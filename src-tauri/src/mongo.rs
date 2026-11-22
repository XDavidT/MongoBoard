//! Talking to MongoDB through the official Rust driver.

use crate::error::{AppError, AppResult};
use crate::profiles::{AuthKind, Profile};
use crate::shell_json;
use crate::tree::{document_node, TreeNode};
use bson::{doc, Bson, Document};
use futures_util::TryStreamExt;
use mongodb::options::{AuthMechanism, ClientOptions, Credential};
use mongodb::Client;
use serde::Serialize;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

/// Open connections, in the order they were opened (the sidebar order).
#[derive(Default)]
pub struct Sessions {
    open: Mutex<Vec<(Uuid, Client, ServerInfo)>>,
}

impl Sessions {
    pub async fn insert(&self, id: Uuid, client: Client, info: ServerInfo) {
        let mut open = self.open.lock().await;
        if let Some(slot) = open.iter_mut().find(|(i, _, _)| *i == id) {
            let old = std::mem::replace(&mut slot.1, client);
            slot.2 = info;
            tokio::spawn(async move { old.shutdown().await });
        } else {
            open.push((id, client, info));
        }
    }

    pub async fn get(&self, id: Uuid) -> AppResult<Client> {
        self.open
            .lock()
            .await
            .iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, c, _)| c.clone())
            .ok_or(AppError::NotConnected)
    }

    pub async fn list(&self) -> Vec<(Uuid, ServerInfo)> {
        self.open
            .lock()
            .await
            .iter()
            .map(|(i, _, info)| (*i, info.clone()))
            .collect()
    }

    pub async fn remove(&self, id: Uuid) {
        let removed = {
            let mut open = self.open.lock().await;
            open.iter()
                .position(|(i, _, _)| *i == id)
                .map(|pos| open.remove(pos))
        };
        if let Some((_, client, _)) = removed {
            client.shutdown().await;
        }
    }
}

/// Build a driver client for a profile. `password` comes from the keychain or the editor form.
pub async fn build_client(profile: &Profile, password: Option<String>) -> AppResult<Client> {
    let mut options = ClientOptions::parse(&profile.uri).await?;
    options.app_name = Some("MongoBoard".into());
    options.server_selection_timeout = Some(Duration::from_secs(15));
    options.connect_timeout = Some(Duration::from_secs(10));

    match profile.auth {
        AuthKind::None => {}
        AuthKind::Password => {
            let username = profile
                .username
                .clone()
                .filter(|u| !u.is_empty())
                .ok_or_else(|| AppError::msg("Username is required"))?;
            let mut credential = options.credential.clone().unwrap_or_default();
            credential.username = Some(username);
            credential.password = password;
            if let Some(source) = profile.auth_source.clone().filter(|s| !s.is_empty()) {
                credential.source = Some(source);
            }
            if let Some(mech) = profile.auth_mechanism.clone().filter(|s| !s.is_empty()) {
                credential.mechanism =
                    Some(AuthMechanism::from_str(&mech).map_err(|e| AppError::msg(e.to_string()))?);
            }
            options.credential = Some(credential);
        }
        AuthKind::Aws => {
            options.credential = Some(aws_credential(profile.aws_profile.as_deref()).await?);
        }
    }

    Ok(Client::with_options(options)?)
}

async fn aws_credential(aws_profile: Option<&str>) -> AppResult<Credential> {
    let mut credential = Credential::default();
    credential.mechanism = Some(AuthMechanism::MongoDbAws);
    credential.source = Some("$external".into());

    // With no named profile the driver resolves AWS credentials itself (env, ~/.aws, SSO, ECS/EC2 role).
    let Some(name) = aws_profile.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(credential);
    };

    use aws_credential_types::provider::ProvideCredentials;
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .profile_name(name)
        .load()
        .await;
    let provider = config
        .credentials_provider()
        .ok_or_else(|| AppError::msg(format!("AWS profile \"{name}\" has no credentials")))?;
    let creds = provider
        .provide_credentials()
        .await
        .map_err(|e| AppError::msg(format!("AWS profile \"{name}\": {e}")))?;
    credential.username = Some(creds.access_key_id().to_string());
    credential.password = Some(creds.secret_access_key().to_string());
    if let Some(token) = creds.session_token() {
        credential.mechanism_properties = Some(doc! { "AWS_SESSION_TOKEN": token });
    }
    Ok(credential)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub version: String,
    pub topology: String,
    pub latency_ms: u128,
}

pub async fn server_info(client: &Client) -> AppResult<ServerInfo> {
    let started = Instant::now();
    let admin = client.database("admin");
    let hello = admin.run_command(doc! { "hello": 1 }).await?;
    let latency_ms = started.elapsed().as_millis();
    let build = admin.run_command(doc! { "buildInfo": 1 }).await?;
    let version = build.get_str("version").unwrap_or("unknown").to_string();
    let topology = if hello.get_str("msg").ok() == Some("isdbgrid") {
        "Sharded cluster (mongos)".to_string()
    } else if let Ok(set) = hello.get_str("setName") {
        let members = hello.get_array("hosts").map(|h| h.len()).unwrap_or(0);
        format!(
            "Replica set {set} · {members} node{}",
            if members == 1 { "" } else { "s" }
        )
    } else {
        "Standalone".to_string()
    };
    Ok(ServerInfo {
        version,
        topology,
        latency_ms,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub name: String,
    pub size_on_disk: u64,
}

pub async fn list_databases(
    client: &Client,
    fallback: Option<&str>,
) -> AppResult<Vec<DatabaseInfo>> {
    match client.list_databases().await {
        Ok(specs) => {
            let mut dbs: Vec<DatabaseInfo> = specs
                .into_iter()
                .map(|s| DatabaseInfo {
                    name: s.name,
                    size_on_disk: s.size_on_disk,
                })
                .collect();
            dbs.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(dbs)
        }
        // Users without listDatabases rights can still browse their default database.
        Err(e) => match fallback.filter(|f| !f.is_empty()) {
            Some(db) => Ok(vec![DatabaseInfo {
                name: db.to_string(),
                size_on_disk: 0,
            }]),
            None => Err(e.into()),
        },
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionInfo {
    pub name: String,
    /// "collection", "view" or "timeseries"
    pub kind: String,
}

pub async fn list_collections(client: &Client, db: &str) -> AppResult<Vec<CollectionInfo>> {
    let specs: Vec<_> = client
        .database(db)
        .list_collections()
        .await?
        .try_collect()
        .await?;
    let mut out: Vec<CollectionInfo> = specs
        .into_iter()
        .map(|s| CollectionInfo {
            name: s.name,
            kind: match s.collection_type {
                mongodb::results::CollectionType::View => "view",
                mongodb::results::CollectionType::Timeseries => "timeseries",
                _ => "collection",
            }
            .to_string(),
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindRequest {
    pub db: String,
    pub collection: String,
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub projection: String,
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub skip: u64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultDoc {
    /// `_id` as a shell literal, used to address this document for edit/delete.
    pub id: Option<String>,
    /// Whole document as editable shell JSON.
    pub shell: String,
    /// Same, without `_id` — the starting point for "Duplicate".
    pub shell_without_id: String,
    pub tree: TreeNode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindResult {
    pub docs: Vec<ResultDoc>,
    pub elapsed_ms: u128,
    pub skip: u64,
    /// True when another page probably exists.
    pub has_more: bool,
}

pub fn to_result_docs(docs: Vec<Document>, offset: usize) -> Vec<ResultDoc> {
    docs.iter()
        .enumerate()
        .map(|(i, d)| ResultDoc {
            id: d.get("_id").map(shell_json::to_shell_inline),
            shell: shell_json::to_shell(d),
            shell_without_id: {
                let mut copy = d.clone();
                copy.remove("_id");
                shell_json::to_shell(&copy)
            },
            tree: document_node(offset + i, d),
        })
        .collect()
}

pub async fn find(client: &Client, req: FindRequest) -> AppResult<FindResult> {
    let filter = shell_json::parse_document(&req.filter)?;
    let projection = shell_json::parse_document(&req.projection)?;
    let sort = shell_json::parse_document(&req.sort)?;
    let limit = req.limit.clamp(1, 10_000);
    let coll = client
        .database(&req.db)
        .collection::<Document>(&req.collection);

    let started = Instant::now();
    let mut action = coll
        .find(filter)
        .skip(req.skip)
        .limit(limit + 1)
        .max_time(QUERY_TIMEOUT);
    if !projection.is_empty() {
        action = action.projection(projection);
    }
    if !sort.is_empty() {
        action = action.sort(sort);
    }
    let mut docs: Vec<Document> = action.await?.try_collect().await?;
    let elapsed_ms = started.elapsed().as_millis();
    let has_more = docs.len() as i64 > limit;
    docs.truncate(limit as usize);
    Ok(FindResult {
        docs: to_result_docs(docs, req.skip as usize),
        elapsed_ms,
        skip: req.skip,
        has_more,
    })
}

pub async fn count(client: &Client, db: &str, collection: &str, filter: &str) -> AppResult<u64> {
    let filter = shell_json::parse_document(filter)?;
    let coll = client.database(db).collection::<Document>(collection);
    Ok(coll.count_documents(filter).max_time(QUERY_TIMEOUT).await?)
}

fn id_filter(id_shell: &str) -> AppResult<Document> {
    let id = shell_json::parse_value(id_shell)?;
    Ok(doc! { "_id": id })
}

/// Insert one document; returns the new `_id` as a shell literal.
pub async fn insert(client: &Client, db: &str, collection: &str, text: &str) -> AppResult<String> {
    let doc = shell_json::parse_document(text)?;
    let coll = client.database(db).collection::<Document>(collection);
    let result = coll.insert_one(doc).await?;
    Ok(shell_json::to_shell_inline(&result.inserted_id))
}

/// Replace the document whose `_id` is `id_shell` with `text`.
pub async fn replace(
    client: &Client,
    db: &str,
    collection: &str,
    id_shell: &str,
    text: &str,
) -> AppResult<u64> {
    let filter = id_filter(id_shell)?;
    let mut doc = shell_json::parse_document(text)?;
    match doc.get("_id") {
        None => {
            doc.insert("_id", filter.get("_id").cloned().unwrap_or(Bson::Null));
        }
        Some(new_id) if Some(new_id) != filter.get("_id") => {
            return Err(AppError::msg(
                "_id can't be changed. Use Duplicate to save a copy with a new _id.",
            ));
        }
        _ => {}
    }
    let coll = client.database(db).collection::<Document>(collection);
    let result = coll.replace_one(filter, doc).await?;
    if result.matched_count == 0 {
        return Err(AppError::msg(
            "The document no longer exists (it may have been deleted)",
        ));
    }
    Ok(result.modified_count)
}

pub async fn delete(client: &Client, db: &str, collection: &str, id_shell: &str) -> AppResult<u64> {
    let filter = id_filter(id_shell)?;
    let coll = client.database(db).collection::<Document>(collection);
    Ok(coll.delete_one(filter).await?.deleted_count)
}

/// System databases that MongoBoard never drops.
pub const PROTECTED_DATABASES: [&str; 3] = ["admin", "local", "config"];

pub async fn drop_database(client: &Client, db: &str) -> AppResult<()> {
    if PROTECTED_DATABASES.contains(&db) {
        return Err(AppError::msg(format!(
            "\"{db}\" is a system database and can't be dropped from MongoBoard"
        )));
    }
    client.database(db).drop().await?;
    Ok(())
}

/// End-to-end tests against a real server. Run with:
/// `MONGODB_URI=mongodb://localhost:27017 cargo test -- --include-ignored`
#[cfg(test)]
mod integration {
    use super::*;
    use crate::profiles::EnvTag;

    fn local_profile(uri: String) -> Profile {
        Profile {
            id: Uuid::new_v4(),
            name: "it".into(),
            uri,
            auth: AuthKind::None,
            username: None,
            auth_source: None,
            auth_mechanism: None,
            aws_profile: None,
            tag: EnvTag::Local,
            read_only: false,
            confirm_prod_writes: true,
            default_database: None,
            last_used: None,
        }
    }

    #[tokio::test]
    #[ignore = "needs MONGODB_URI"]
    async fn crud_round_trip() {
        let uri = std::env::var("MONGODB_URI").expect("set MONGODB_URI");
        let client = build_client(&local_profile(uri), None).await.unwrap();
        let info = server_info(&client).await.unwrap();
        assert!(!info.version.is_empty());

        let db = format!("mongoboard_it_{}", Uuid::new_v4().simple());
        let coll = "orders";

        let id = insert(&client, &db, coll, r#"{ orderNo: "A-1", status: "new", stops: [ { zip: "M5V" } ], eta: ISODate("2026-11-19T14:30:00Z"), n: NumberLong(5) }"#)
            .await
            .unwrap();
        assert!(id.starts_with("ObjectId(\""));
        insert(&client, &db, coll, r#"{ orderNo: "A-2", status: "done" }"#)
            .await
            .unwrap();

        let dbs = list_databases(&client, None).await.unwrap();
        assert!(dbs.iter().any(|d| d.name == db));
        let cols = list_collections(&client, &db).await.unwrap();
        assert_eq!(cols.len(), 1);

        let page = find(
            &client,
            FindRequest {
                db: db.clone(),
                collection: coll.into(),
                filter: "{ status: 'new' }".into(),
                projection: String::new(),
                sort: "{ orderNo: 1 }".into(),
                skip: 0,
                limit: 50,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.docs.len(), 1);
        assert!(!page.has_more);
        let doc = &page.docs[0];
        assert_eq!(doc.id.as_deref(), Some(id.as_str()));
        assert!(doc.shell.contains("n: NumberLong(5)"), "{}", doc.shell);
        assert_eq!(count(&client, &db, coll, "{}").await.unwrap(), 2);

        // Paging: limit 1 over 2 docs reports has_more.
        let first = find(
            &client,
            FindRequest {
                db: db.clone(),
                collection: coll.into(),
                filter: String::new(),
                projection: String::new(),
                sort: "{ orderNo: 1 }".into(),
                skip: 0,
                limit: 1,
            },
        )
        .await
        .unwrap();
        assert!(first.has_more);

        // Edit: change status, keep types.
        let edited = doc
            .shell
            .replace("status: \"new\"", "status: \"out_for_delivery\"");
        assert_eq!(replace(&client, &db, coll, &id, &edited).await.unwrap(), 1);
        let again = find(
            &client,
            FindRequest {
                db: db.clone(),
                collection: coll.into(),
                filter: format!("{{ _id: {id} }}"),
                projection: String::new(),
                sort: String::new(),
                skip: 0,
                limit: 5,
            },
        )
        .await
        .unwrap();
        assert!(again.docs[0].shell.contains("out_for_delivery"));
        assert!(again.docs[0].shell.contains("n: NumberLong(5)"));

        // Changing _id is refused.
        let bad = edited.replace(&id, "ObjectId(\"000000000000000000000000\")");
        assert!(replace(&client, &db, coll, &id, &bad).await.is_err());

        // Duplicate = insert the copy without _id.
        insert(&client, &db, coll, &again.docs[0].shell_without_id)
            .await
            .unwrap();
        assert_eq!(
            count(&client, &db, coll, "{ orderNo: 'A-1' }")
                .await
                .unwrap(),
            2
        );

        // Delete one.
        assert_eq!(delete(&client, &db, coll, &id).await.unwrap(), 1);
        assert_eq!(count(&client, &db, coll, "{}").await.unwrap(), 2);

        // Drop database: system databases are refused, ours goes away.
        assert!(drop_database(&client, "admin").await.is_err());
        drop_database(&client, &db).await.unwrap();
        let after = list_databases(&client, None).await.unwrap();
        assert!(!after.iter().any(|d| d.name == db));
    }
}
