//! Tauri commands: the API the React frontend calls with `invoke(...)`.

use crate::error::{AppError, AppResult};
use crate::mongo::{
    self, CollectionInfo, DatabaseInfo, FindRequest, FindResult, ServerInfo, Sessions,
};
use crate::profiles::{split_credentials, AuthKind, Profile, ProfileStore};
use crate::secrets;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

pub const PROFILES_CHANGED: &str = "profiles-changed";
pub const CONNECTIONS_CHANGED: &str = "connections-changed";

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------- profiles

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    #[serde(flatten)]
    pub profile: Profile,
    pub has_password: bool,
}

fn view(p: Profile) -> ProfileView {
    let has_password =
        p.auth == AuthKind::Password && secrets::get_password(p.id).ok().flatten().is_some();
    ProfileView {
        profile: p,
        has_password,
    }
}

#[tauri::command]
pub fn list_profiles(store: State<'_, ProfileStore>) -> Vec<ProfileView> {
    let mut list = store.list();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list.into_iter().map(view).collect()
}

#[tauri::command]
pub fn get_profile(store: State<'_, ProfileStore>, id: Uuid) -> AppResult<ProfileView> {
    store.get(id).map(view).ok_or(AppError::UnknownProfile)
}

/// What the connection editor submits.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    #[serde(flatten)]
    pub profile: Profile,
    /// `None` keeps the stored password; `Some("")` clears it.
    pub password: Option<String>,
}

/// Normalise editor input: move credentials embedded in the URI into the profile/keychain fields.
fn normalise(mut input: ProfileInput) -> (Profile, Option<String>) {
    let split = split_credentials(&input.profile.uri);
    input.profile.uri = split.uri;
    if split.username.is_some() || split.password.is_some() {
        if input.profile.auth == AuthKind::None {
            input.profile.auth = AuthKind::Password;
        }
        if input.profile.username.as_deref().unwrap_or("").is_empty() {
            input.profile.username = split.username;
        }
        if input.password.as_deref().unwrap_or("").is_empty() && split.password.is_some() {
            input.password = split.password;
        }
    }
    input.profile.name = input.profile.name.trim().to_string();
    if input.profile.name.is_empty() {
        input.profile.name = "Untitled connection".into();
    }
    (input.profile, input.password)
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    store: State<'_, ProfileStore>,
    input: ProfileInput,
) -> AppResult<ProfileView> {
    let (profile, password) = normalise(input);
    if profile.uri.trim().is_empty() {
        return Err(AppError::msg("Connection string is required"));
    }
    match (&profile.auth, password) {
        (AuthKind::Password, Some(p)) if p.is_empty() => secrets::delete_password(profile.id)?,
        (AuthKind::Password, Some(p)) => secrets::set_password(profile.id, &p)?,
        (AuthKind::Password, None) => {}
        // No password needed any more: remove a stale one, best effort.
        _ => {
            let _ = secrets::delete_password(profile.id);
        }
    }
    store.upsert(profile.clone())?;
    let _ = app.emit(PROFILES_CHANGED, ());
    Ok(view(profile))
}

#[tauri::command]
pub fn duplicate_profile(
    app: AppHandle,
    store: State<'_, ProfileStore>,
    id: Uuid,
) -> AppResult<ProfileView> {
    let mut copy = store.get(id).ok_or(AppError::UnknownProfile)?;
    copy.id = Uuid::new_v4();
    copy.name = format!("{} copy", copy.name);
    copy.last_used = None;
    if let Some(pw) = secrets::get_password(id)? {
        secrets::set_password(copy.id, &pw)?;
    }
    store.upsert(copy.clone())?;
    let _ = app.emit(PROFILES_CHANGED, ());
    Ok(view(copy))
}

#[tauri::command]
pub async fn delete_profile(
    app: AppHandle,
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
) -> AppResult<()> {
    sessions.remove(id).await;
    secrets::delete_password(id)?;
    store.remove(id)?;
    let _ = app.emit(CONNECTIONS_CHANGED, ());
    let _ = app.emit(PROFILES_CHANGED, ());
    Ok(())
}

/// "Test" button: connect with the unsaved form values.
#[tauri::command]
pub async fn test_connection(input: ProfileInput) -> AppResult<ServerInfo> {
    let id = input.profile.id;
    let (profile, password) = normalise(input);
    let password = match password {
        Some(p) if !p.is_empty() => Some(p),
        _ => secrets::get_password(id).ok().flatten(),
    };
    let client = mongo::build_client(&profile, password).await?;
    let info = mongo::server_info(&client).await;
    client.shutdown().await;
    info
}

// ---------------------------------------------------------------- windows

fn focus_or_build(
    app: &AppHandle,
    label: &str,
    url: String,
    title: &str,
    size: (f64, f64),
    min: (f64, f64),
) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(size.0, size.1)
        .min_inner_size(min.0, min.1)
        .center();
    // Dialog-style windows stay above the main window.
    if let Some(main) = app.get_webview_window("main") {
        builder = builder
            .parent(&main)
            .map_err(|e| AppError::msg(e.to_string()))?;
    }
    builder.build().map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

fn focus_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

#[tauri::command]
pub async fn open_connection_manager(app: AppHandle) -> AppResult<()> {
    focus_or_build(
        &app,
        "manager",
        "index.html?view=manager".into(),
        "Connection Manager",
        (1000.0, 640.0),
        (760.0, 420.0),
    )
}

/// Open the editor for an existing profile, or for a new one when `id` is None.
#[tauri::command]
pub async fn open_connection_editor(app: AppHandle, id: Option<Uuid>) -> AppResult<()> {
    let (label, url, title) = match id {
        Some(id) => (
            format!("editor-{}", id.simple()),
            format!("index.html?view=editor&id={id}"),
            "Edit connection",
        ),
        None => {
            let new_id = Uuid::new_v4();
            (
                format!("editor-{}", new_id.simple()),
                format!("index.html?view=editor&new={new_id}"),
                "New connection",
            )
        }
    };
    focus_or_build(&app, &label, url, title, (760.0, 720.0), (620.0, 560.0))
}

async fn connect_profile(
    store: &ProfileStore,
    sessions: &Sessions,
    profile: &Profile,
) -> AppResult<ServerInfo> {
    let password = if profile.auth == AuthKind::Password {
        secrets::get_password(profile.id)?
    } else {
        None
    };
    let client = mongo::build_client(profile, password).await?;
    let info = mongo::server_info(&client).await?;
    sessions.insert(profile.id, client, info.clone()).await;
    store.touch(profile.id, now_millis());
    Ok(info)
}

/// Connect a saved profile and add it to the main window's sidebar.
#[tauri::command]
pub async fn open_connection(
    app: AppHandle,
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
) -> AppResult<ServerInfo> {
    let profile = store.get(id).ok_or(AppError::UnknownProfile)?;
    let info = match sessions.list().await.into_iter().find(|(i, _)| *i == id) {
        Some((_, info)) => info,
        None => connect_profile(&store, &sessions, &profile).await?,
    };
    let _ = app.emit(CONNECTIONS_CHANGED, ());
    let _ = app.emit(PROFILES_CHANGED, ());
    focus_main(&app);
    Ok(info)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenConnection {
    pub profile: ProfileView,
    pub server: ServerInfo,
}

#[tauri::command]
pub async fn list_open_connections(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
) -> AppResult<Vec<OpenConnection>> {
    Ok(sessions
        .list()
        .await
        .into_iter()
        .filter_map(|(id, server)| {
            store.get(id).map(|p| OpenConnection {
                profile: view(p),
                server,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn close_connection(
    app: AppHandle,
    sessions: State<'_, Sessions>,
    id: Uuid,
) -> AppResult<()> {
    sessions.remove(id).await;
    let _ = app.emit(CONNECTIONS_CHANGED, ());
    Ok(())
}

// ---------------------------------------------------------------- browsing

/// Reconnect an open connection, e.g. after a network drop.
#[tauri::command]
pub async fn reconnect(
    app: AppHandle,
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
) -> AppResult<ServerInfo> {
    let profile = store.get(id).ok_or(AppError::UnknownProfile)?;
    let info = connect_profile(&store, &sessions, &profile).await?;
    let _ = app.emit(CONNECTIONS_CHANGED, ());
    Ok(info)
}

#[tauri::command]
pub async fn list_databases(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
) -> AppResult<Vec<DatabaseInfo>> {
    let profile = store.get(id).ok_or(AppError::UnknownProfile)?;
    let client = sessions.get(id).await?;
    mongo::list_databases(&client, profile.default_database.as_deref()).await
}

#[tauri::command]
pub async fn list_collections(
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
) -> AppResult<Vec<CollectionInfo>> {
    mongo::list_collections(&sessions.get(id).await?, &db).await
}

#[tauri::command]
pub async fn find(
    sessions: State<'_, Sessions>,
    id: Uuid,
    request: FindRequest,
) -> AppResult<FindResult> {
    mongo::find(&sessions.get(id).await?, request).await
}

#[tauri::command]
pub async fn count_documents(
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
    collection: String,
    filter: String,
) -> AppResult<u64> {
    mongo::count(&sessions.get(id).await?, &db, &collection, &filter).await
}

/// Split a pasted connection string into editor fields (no network access).
#[tauri::command]
pub fn parse_connection_string(uri: String) -> crate::profiles::ParsedUri {
    crate::profiles::parse_connection_string(&uri)
}

/// Parse + reformat shell JSON without touching the database (editor "Format"/validation).
#[tauri::command]
pub fn format_document(text: String) -> AppResult<String> {
    Ok(crate::shell_json::to_shell(
        &crate::shell_json::parse_document(&text)?,
    ))
}

// ---------------------------------------------------------------- writes

fn writable(store: &ProfileStore, id: Uuid) -> AppResult<()> {
    let profile = store.get(id).ok_or(AppError::UnknownProfile)?;
    if profile.writes_allowed() {
        Ok(())
    } else {
        Err(AppError::ReadOnly)
    }
}

#[tauri::command]
pub async fn insert_document(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
    collection: String,
    text: String,
) -> AppResult<String> {
    writable(&store, id)?;
    mongo::insert(&sessions.get(id).await?, &db, &collection, &text).await
}

#[tauri::command]
pub async fn replace_document(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
    collection: String,
    doc_id: String,
    text: String,
) -> AppResult<u64> {
    writable(&store, id)?;
    mongo::replace(&sessions.get(id).await?, &db, &collection, &doc_id, &text).await
}

#[tauri::command]
pub async fn delete_document(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
    collection: String,
    doc_id: String,
) -> AppResult<u64> {
    writable(&store, id)?;
    mongo::delete(&sessions.get(id).await?, &db, &collection, &doc_id).await
}

#[tauri::command]
pub async fn drop_database(
    store: State<'_, ProfileStore>,
    sessions: State<'_, Sessions>,
    id: Uuid,
    db: String,
) -> AppResult<()> {
    writable(&store, id)?;
    mongo::drop_database(&sessions.get(id).await?, &db).await
}
