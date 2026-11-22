//! Saved connection profiles. Stored as JSON in the app's config folder; passwords live
//! in the OS keychain (see `secrets.rs`) and are never written to this file.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EnvTag {
    Production,
    Qa,
    Dev,
    Local,
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    /// No credentials, or whatever the connection string itself carries.
    #[default]
    None,
    /// Username + password (SCRAM). Password is kept in the keychain.
    Password,
    /// MONGODB-AWS: IAM role / AWS credentials, optionally from a named AWS profile.
    Aws,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    /// Connection string without username/password.
    pub uri: String,
    #[serde(default)]
    pub auth: AuthKind,
    #[serde(default)]
    pub username: Option<String>,
    /// authSource override (defaults to the URI's, then "admin").
    #[serde(default)]
    pub auth_source: Option<String>,
    /// SCRAM-SHA-1 / SCRAM-SHA-256; empty = let the server negotiate.
    #[serde(default)]
    pub auth_mechanism: Option<String>,
    /// Named profile from ~/.aws/config; empty = default AWS credential chain.
    #[serde(default)]
    pub aws_profile: Option<String>,
    #[serde(default)]
    pub tag: EnvTag,
    #[serde(default)]
    pub read_only: bool,
    /// Ask for confirmation before any write when tagged Production.
    #[serde(default = "default_true")]
    pub confirm_prod_writes: bool,
    #[serde(default)]
    pub default_database: Option<String>,
    #[serde(default)]
    pub last_used: Option<i64>,
}

fn default_true() -> bool {
    true
}

impl Profile {
    pub fn writes_allowed(&self) -> bool {
        !self.read_only
    }
}

pub struct ProfileStore {
    path: PathBuf,
    profiles: Mutex<Vec<Profile>>,
}

impl ProfileStore {
    pub fn load(path: PathBuf) -> Self {
        let profiles = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<Profile>>(&s).ok())
            .unwrap_or_default();
        Self {
            path,
            profiles: Mutex::new(profiles),
        }
    }

    pub fn list(&self) -> Vec<Profile> {
        self.profiles.lock().unwrap().clone()
    }

    pub fn get(&self, id: Uuid) -> Option<Profile> {
        self.profiles
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    pub fn upsert(&self, profile: Profile) -> std::io::Result<()> {
        let mut list = self.profiles.lock().unwrap();
        match list.iter_mut().find(|p| p.id == profile.id) {
            Some(existing) => *existing = profile,
            None => list.push(profile),
        }
        self.persist(&list)
    }

    pub fn remove(&self, id: Uuid) -> std::io::Result<()> {
        let mut list = self.profiles.lock().unwrap();
        list.retain(|p| p.id != id);
        self.persist(&list)
    }

    pub fn touch(&self, id: Uuid, at_millis: i64) {
        let mut list = self.profiles.lock().unwrap();
        if let Some(p) = list.iter_mut().find(|p| p.id == id) {
            p.last_used = Some(at_millis);
        }
        let _ = self.persist(&list);
    }

    fn persist(&self, list: &[Profile]) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(list)?)?;
        std::fs::rename(tmp, &self.path)
    }
}

/// v0.1.0 shipped as "MongBoard" (bundle id `io.mongboard.app`). Copy its saved
/// connections into the new config folder the first time MongoBoard starts.
pub fn migrate_legacy_file(path: &std::path::Path) {
    if path.exists() {
        return;
    }
    let Some(new_dir) = path.parent() else { return };
    let Some(parent) = new_dir.parent() else {
        return;
    };
    let legacy = parent.join("io.mongboard.app").join("connections.json");
    if legacy.exists() {
        let _ = std::fs::create_dir_all(new_dir);
        let _ = std::fs::copy(&legacy, path);
    }
}

/// Credentials pulled out of a connection string.
#[derive(Debug, PartialEq, Eq)]
pub struct SplitUri {
    pub uri: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Remove `user:pass@` from a MongoDB URI so the password never lands in the profile file.
/// Works for multi-host URIs (which are not valid URLs).
pub fn split_credentials(uri: &str) -> SplitUri {
    let trimmed = uri.trim();
    let Some(scheme_end) = trimmed.find("://") else {
        return SplitUri {
            uri: trimmed.to_string(),
            username: None,
            password: None,
        };
    };
    let rest = &trimmed[scheme_end + 3..];
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let Some(at) = authority.rfind('@') else {
        return SplitUri {
            uri: trimmed.to_string(),
            username: None,
            password: None,
        };
    };
    let userinfo = &authority[..at];
    let (user, pass) = match userinfo.split_once(':') {
        Some((u, p)) => (u, Some(p)),
        None => (userinfo, None),
    };
    let clean = format!(
        "{}{}{}",
        &trimmed[..scheme_end + 3],
        &authority[at + 1..],
        &rest[authority_end..]
    );
    SplitUri {
        uri: clean,
        username: Some(percent_decode(user)).filter(|s| !s.is_empty()),
        password: pass.map(percent_decode),
    }
}

/// What we could pull out of a pasted connection string.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedUri {
    /// Cleaned connection string: no credentials, no auth options, no Studio 3T settings.
    pub uri: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auth: Option<AuthKind>,
    pub auth_source: Option<String>,
    pub auth_mechanism: Option<String>,
    pub aws_profile: Option<String>,
    pub default_database: Option<String>,
    /// Guessed from the connection name (prod / qa / dev / localhost).
    pub tag: Option<EnvTag>,
    /// Human-readable list of what was moved or dropped.
    pub notes: Vec<String>,
}

/// Split a pasted connection string (including Studio 3T / Compass exports) into profile fields.
/// Credentials and auth options move to their own fields; `3t.*` settings are dropped;
/// everything the driver understands stays in the URI.
pub fn parse_connection_string(input: &str) -> ParsedUri {
    let split = split_credentials(input);
    let mut out = ParsedUri {
        username: split.username,
        password: split.password,
        ..Default::default()
    };
    if out.username.is_some() {
        out.notes.push("username".into());
    }
    if out.password.is_some() {
        out.notes.push("password → Keychain".into());
    }

    let uri = split.uri;
    let (base, query) = match uri.split_once('?') {
        Some((b, q)) => (b.to_string(), q.to_string()),
        None => (uri.clone(), String::new()),
    };

    // Path after the hosts = default/auth database.
    let mut base = base;
    if let Some(scheme_end) = base.find("://") {
        if let Some(slash) = base[scheme_end + 3..].find('/') {
            let at = scheme_end + 3 + slash;
            let db = percent_decode(&base[at + 1..]);
            base.truncate(at);
            if !db.is_empty() {
                out.default_database = Some(db);
            }
        }
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut dropped_3t = 0;
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(&value.replace('+', " "));
        let lower = key.to_ascii_lowercase();
        match lower.as_str() {
            "authsource" => out.auth_source = Some(value),
            "authmechanism" => match value.to_ascii_uppercase().as_str() {
                "SCRAM-SHA-1" | "SCRAM-SHA-256" => {
                    out.auth_mechanism = Some(value.to_ascii_uppercase())
                }
                "MONGODB-AWS" => out.auth = Some(AuthKind::Aws),
                // Mechanisms the editor has no fields for stay in the URI.
                _ => kept.push(pair),
            },
            "authmechanismproperties"
                if out.auth == Some(AuthKind::Aws) || value.contains("AWS") =>
            {
                // AWS_PROFILE:name is a MongoBoard convenience; session tokens come from the credential chain.
                for prop in value.split(',') {
                    if let Some(p) = prop.strip_prefix("AWS_PROFILE:") {
                        out.aws_profile = Some(p.to_string());
                    }
                }
            }
            "3t.connection.name" => out.name = Some(value).filter(|v| !v.is_empty()),
            k if k.starts_with("3t.") => dropped_3t += 1,
            _ => kept.push(pair),
        }
    }

    if out.auth.is_none() && (out.username.is_some() || out.password.is_some()) {
        out.auth = Some(AuthKind::Password);
    }
    if out.auth == Some(AuthKind::Aws) {
        // Credentials come from AWS, not from the URI.
        out.username = None;
        out.password = None;
        out.auth_source = None;
        out.notes
            .retain(|n| !n.starts_with("username") && !n.starts_with("password"));
    }
    // With password auth and no explicit authSource, the path database is where the user lives.
    if out.auth == Some(AuthKind::Password) && out.auth_source.is_none() {
        out.auth_source = out.default_database.clone();
    }
    // "admin" as the path is almost always just the auth database, not a database to open.
    if out.default_database.as_deref() == Some("admin") {
        out.default_database = None;
    }

    if out.auth_source.is_some() {
        out.notes.push("auth database".into());
    }
    if out.auth_mechanism.is_some() {
        out.notes.push("mechanism".into());
    }
    if out.auth == Some(AuthKind::Aws) {
        out.notes.push("AWS IAM".into());
    }
    if let Some(name) = &out.name {
        out.notes.push("name".into());
        let n = name.to_ascii_lowercase();
        let is_non_prod = n.contains("non-prod") || n.contains("nonprod") || n.contains("non_prod");
        out.tag = if n.contains("prod") && !is_non_prod {
            Some(EnvTag::Production)
        } else if n.contains("qa") || n.contains("stag") || n.contains("uat") || is_non_prod {
            Some(EnvTag::Qa)
        } else if n.contains("dev") {
            Some(EnvTag::Dev)
        } else {
            None
        };
    }
    if base.contains("localhost") || base.contains("127.0.0.1") {
        out.tag = Some(EnvTag::Local);
    }
    if dropped_3t > 0 {
        out.notes.push(format!(
            "removed {dropped_3t} Studio 3T setting{}",
            if dropped_3t == 1 { "" } else { "s" }
        ));
    }

    out.uri = if kept.is_empty() {
        format!("{base}/")
    } else {
        format!("{base}/?{}", kept.join("&"))
    };
    out
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(v) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_credentials_from_srv_uri() {
        let s = split_credentials(
            "mongodb+srv://app%40ro:p%2Fss:w@cluster0.abc.mongodb.net/deliveries?retryWrites=true",
        );
        assert_eq!(
            s.uri,
            "mongodb+srv://cluster0.abc.mongodb.net/deliveries?retryWrites=true"
        );
        assert_eq!(s.username.as_deref(), Some("app@ro"));
        assert_eq!(s.password.as_deref(), Some("p/ss:w"));
    }

    #[test]
    fn handles_multi_host_and_no_credentials() {
        let s = split_credentials("mongodb://u:p@h1:27017,h2:27017/?replicaSet=rs0");
        assert_eq!(s.uri, "mongodb://h1:27017,h2:27017/?replicaSet=rs0");
        assert_eq!(s.username.as_deref(), Some("u"));
        let none = split_credentials("mongodb://localhost:27017");
        assert_eq!(
            none,
            SplitUri {
                uri: "mongodb://localhost:27017".into(),
                username: None,
                password: None
            }
        );
        let user_only = split_credentials("mongodb://bob@localhost");
        assert_eq!(user_only.username.as_deref(), Some("bob"));
        assert_eq!(user_only.password, None);
    }

    #[test]
    fn parses_studio_3t_export() {
        let p = parse_connection_string(
            "mongodb+srv://david:s3cr%40t@light-pl-0.0ogdx.mongodb.net/admin?retryWrites=true&loadBalanced=false&replicaSet=atlas-gsn65o-shard-0&readPreference=primary&srvServiceName=mongodb&connectTimeoutMS=10000&authSource=admin&authMechanism=SCRAM-SHA-1&3t.uriVersion=3&3t.connection.name=Light-PL&3t.databases=admin&3t.alwaysShowAuthDB=true&3t.alwaysShowDBFromUserRole=true&3t.sslTlsVersion=TLS",
        );
        assert_eq!(
            p.uri,
            "mongodb+srv://light-pl-0.0ogdx.mongodb.net/?retryWrites=true&loadBalanced=false&replicaSet=atlas-gsn65o-shard-0&readPreference=primary&srvServiceName=mongodb&connectTimeoutMS=10000"
        );
        assert_eq!(p.name.as_deref(), Some("Light-PL"));
        assert_eq!(p.username.as_deref(), Some("david"));
        assert_eq!(p.password.as_deref(), Some("s3cr@t"));
        assert_eq!(p.auth, Some(AuthKind::Password));
        assert_eq!(p.auth_source.as_deref(), Some("admin"));
        assert_eq!(p.auth_mechanism.as_deref(), Some("SCRAM-SHA-1"));
        assert_eq!(p.default_database, None);
        assert!(
            p.notes.iter().any(|n| n.contains("5 Studio 3T")),
            "{:?}",
            p.notes
        );
    }

    #[test]
    fn parses_other_shapes() {
        let aws = parse_connection_string("mongodb+srv://c.example.net/?authMechanism=MONGODB-AWS&authSource=$external&authMechanismProperties=AWS_PROFILE:dev");
        assert_eq!(aws.auth, Some(AuthKind::Aws));
        assert_eq!(aws.aws_profile.as_deref(), Some("dev"));
        assert_eq!(aws.uri, "mongodb+srv://c.example.net/");

        let plain =
            parse_connection_string("mongodb://bob:pw@h1:27017,h2:27017/deliveries?replicaSet=rs0");
        assert_eq!(plain.uri, "mongodb://h1:27017,h2:27017/?replicaSet=rs0");
        assert_eq!(plain.default_database.as_deref(), Some("deliveries"));
        assert_eq!(plain.auth_source.as_deref(), Some("deliveries"));

        let local = parse_connection_string("mongodb://localhost:27017");
        assert_eq!(local.uri, "mongodb://localhost:27017/");
        assert_eq!(local.tag, Some(EnvTag::Local));
        assert_eq!(local.auth, None);

        assert_eq!(
            parse_connection_string("mongodb+srv://a/?3t.connection.name=NON-PROD").tag,
            Some(EnvTag::Qa)
        );
        assert_eq!(
            parse_connection_string("mongodb+srv://a/?3t.connection.name=prod-us").tag,
            Some(EnvTag::Production)
        );
    }

    #[test]
    fn migrates_legacy_config() {
        let root = std::env::temp_dir().join(format!("mongoboard-mig-{}", Uuid::new_v4()));
        let legacy = root.join("io.mongboard.app");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("connections.json"), "[]").unwrap();
        let path = root.join("io.mongoboard.app").join("connections.json");
        migrate_legacy_file(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[]");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn store_round_trip() {
        let dir = std::env::temp_dir().join(format!("mongoboard-test-{}", Uuid::new_v4()));
        let path = dir.join("connections.json");
        let store = ProfileStore::load(path.clone());
        let p = Profile {
            id: Uuid::new_v4(),
            name: "local".into(),
            uri: "mongodb://localhost".into(),
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
        };
        store.upsert(p.clone()).unwrap();
        let reloaded = ProfileStore::load(path);
        assert_eq!(reloaded.list(), vec![p.clone()]);
        reloaded.remove(p.id).unwrap();
        assert!(reloaded.list().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
