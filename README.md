<div align="center">

<img src="docs/logo.png" alt="MongoBoard logo" width="112" height="112">

# MongoBoard

**A fast, native-feeling MongoDB client for macOS.**
Browse collections in a Studio 3T–style tree, edit documents safely, and connect with a password *or* your AWS IAM role.

🆓 A free, open-source **Studio 3T alternative**, **Robo 3T alternative** and **NoSQLBooster alternative**.

[![Latest release](https://img.shields.io/github/v/release/XDavidT/MongoBoard?label=release&color=0f8a5a)](https://github.com/XDavidT/MongoBoard/releases/latest)
[![CI](https://github.com/XDavidT/MongoBoard/actions/workflows/ci.yml/badge.svg)](https://github.com/XDavidT/MongoBoard/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%2012%2B-lightgrey?logo=apple)](https://github.com/XDavidT/MongoBoard/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/XDavidT/MongoBoard/total?color=0f8a5a)](https://github.com/XDavidT/MongoBoard/releases)

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-official%20MongoDB%20driver-000000?logo=rust)](https://github.com/mongodb/mongo-rust-driver)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)

[**⬇️ Download for macOS**](https://github.com/XDavidT/MongoBoard/releases/latest) · [✨ Features](#-features) · [📸 Screenshots](#-screenshots) · [🗺️ Roadmap](#%EF%B8%8F-roadmap) · [🤝 Contributing](CONTRIBUTING.md)

<br>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-window-dark.png">
  <img alt="MongoBoard main window: open connections in the sidebar and a collection in the tree view" src="docs/screenshots/main-window-light.png" width="100%">
</picture>

</div>

---

## 🔄 Looking for a Studio 3T, Robo 3T or NoSQLBooster alternative?

MongoBoard covers the everyday work those GUIs are used for: saved connections, a tree view of documents, filtering, and safe single-document edits. It's free, open source (MIT), and built for macOS.

| | MongoBoard |
|---|---|
| 💸 Price | Free, MIT licensed |
| 🌲 Studio 3T-style tree view (Key / Value / Type) | ✅ |
| ☁️ AWS IAM (`MONGODB-AWS`) sign-in | ✅ |
| 🔑 Passwords in the macOS Keychain | ✅ |
| 🛡️ Read-only connections + Production write guard | ✅ |
| 🍎 Native macOS app, Apple Silicon + Intel, light/dark | ✅ |
| 🧱 Aggregation builder, indexes, import/export | 🚧 [On the roadmap](#%EF%B8%8F-roadmap) |

## ✨ Features

### 🪟 One window, many connections
- 🧭 **Open connections in the sidebar**: each connection expands into its databases and collections, with several clusters open side by side
- 🗂️ **Collections open as tabs**, each keeping its own query, page and selection
- 🏠 **Quickstart tab** with your recent connections, one click to reconnect
- 🔍 Search across everything that's open

### 🔌 Connections
- 📇 **Connection Manager** pop-up (`⌘⇧C`) to connect, add, edit, duplicate or delete; connections are grouped by environment (Production / Non-production / Local)
- 🔗 Paste any `mongodb://` or `mongodb+srv://` string, **even a Studio 3T or Compass export**: user, password, auth settings and name are filled in for you, and tool-specific junk is stripped. Atlas, replica sets and sharded clusters all work
- 🔑 **Username & password (SCRAM)**: passwords live in the **macOS Keychain**, never in a config file
- ☁️ **AWS IAM (`MONGODB-AWS`)**: use your default AWS credential chain, SSO, an attached role, or any named profile from `~/.aws`
- 🧪 **Test before you save**: shows server version, topology and latency
- 🏷️ **Environment tags** (Production · QA · Dev · Local) shown on every window, so you always know where you are

### 🌳 Browse
- 📚 Database → collection sidebar with sizes, views and time-series collections
- 🌲 **Tree view**: Key / Value / Type columns, expandable nested objects and arrays, type badges, colour-coded values
- `{ }` **JSON view** in mongosh-style shell syntax
- 🔎 **Filter, projection, sort and limit** with full shell syntax: `ObjectId("…")`, `ISODate("…")`, `/regex/i`, unquoted keys
- 📄 Paging with exact counts and query timing

### ✏️ Edit safely
- ➕ **Add**, ✏️ **Edit**, 📑 **Duplicate** and 🗑️ **Delete**, one selected document at a time
- 🧬 **Type-preserving editor**: `Int32`, `Int64`, `Double`, `Date`, `ObjectId`, `Decimal128`, `UUID`… round-trip exactly
- 🛡️ **Read-only connections** block every write in the backend, not just the UI
- 🚨 **Production guard**: writes to a Production-tagged connection need a second, explicit confirmation
- 🙅 `_id` can't be changed by accident (use Duplicate instead)
- 🗑️ **Drop database** with type-the-name confirmation; system databases (`admin`, `local`, `config`) are protected

### 🍎 Feels like a Mac app
- 🌗 **Light & dark mode** follow your macOS appearance automatically
- ⌨️ Menu bar and shortcuts: `⌘N` new connection, `⌘⇧C` Connection Manager, `⌘↵` run query, `⌘E` edit, `⌘S` save
- 🔄 **Self-updating**: new versions download in the background from GitHub Releases (signed and verified), then one click to restart
- 🪶 Small and fast: ~16 MB universal build (Apple Silicon + Intel), no Electron

## 📸 Screenshots

**Quickstart**: recent connections and shortcuts, with open connections in the sidebar:

<picture><source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/quickstart-dark.png"><img src="docs/screenshots/quickstart-light.png" alt="Quickstart tab" width="100%"></picture>

| Connection Manager | Connection editor |
|:---:|:---:|
| <picture><source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/connection-manager-dark.png"><img src="docs/screenshots/connection-manager-light.png" alt="Connection Manager"></picture> | <picture><source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/connection-editor-dark.png"><img src="docs/screenshots/connection-editor-light.png" alt="Connection editor"></picture> |

**Editing a document**, with shell syntax and types preserved:

<img src="docs/screenshots/edit-document-light.png" alt="Edit document panel" width="100%">

<sub>Screenshots use sample data. They follow your GitHub theme (light/dark), just like the app follows macOS.</sub>

## ⬇️ Install

1. Download **`MongoBoard_x.y.z_universal.dmg`** from the [**latest release**](https://github.com/XDavidT/MongoBoard/releases/latest).
2. Open the DMG and drag **MongoBoard** into **Applications**.
3. First launch: the app isn't notarized yet, so **right-click → Open** once.
   If macOS says the app is "damaged", clear the download flag:
   ```sh
   xattr -dr com.apple.quarantine /Applications/MongoBoard.app
   ```

> [!NOTE]
> Requires macOS 12 Monterey or later. Works on Apple Silicon and Intel.

## 🚀 Quick start

1. **New connection** (`⌘N`) → paste your connection string (a Studio 3T export works too: the fields fill in by themselves).
2. Pick **Authentication**: *Username and password*, *AWS IAM*, or *None*.
3. Tag it (e.g. **Production**) and tick **Read-only** if you only want to look.
4. **Test** → **Save & connect**. The connection appears in the sidebar; click a collection to open it in a tab.
5. Next time, reconnect from the **Quickstart** tab or **Connect…** (`⌘⇧C`).

<details>
<summary><b>☁️ Using AWS IAM with Atlas</b></summary>

1. In Atlas, add a **database user** with authentication method **AWS IAM** and the ARN of your role or user.
2. In MongoBoard, choose **AWS IAM** and either leave *AWS profile* empty (uses the default credential chain: env vars, `~/.aws`, SSO, instance role) or type a profile name from `~/.aws/config`.
3. Your connection string doesn't need a username or password. MongoBoard sets `authMechanism=MONGODB-AWS` for you.

If you use SSO, run `aws sso login --profile <name>` first.
</details>

## 🗺️ Roadmap

- [x] Main window with open-connections sidebar, collection tabs and Quickstart
- [x] Connection Manager + editor pop-ups, Keychain storage
- [x] Password (SCRAM) and AWS IAM authentication
- [x] Tree & JSON views, filter / projection / sort / paging
- [x] Add / edit / duplicate / delete with read-only + production guards
- [x] Light & dark mode
- [ ] 🧱 Aggregation pipeline builder (stage-by-stage preview, copy as code)
- [ ] 📇 Indexes, collection stats and visual `explain()`
- [ ] 📋 Table view with dynamic columns
- [ ] 📦 Import / export (JSON, CSV)
- [ ] 🧾 Field-by-field document editor
- [ ] 🚇 SSH tunnel and X.509 certificates
- [x] 🔄 Automatic, signed updates
- [ ] ✍️ Notarized builds (no right-click → Open on first launch)

Have an idea? [Open a feature request](https://github.com/XDavidT/MongoBoard/issues/new?template=feature_request.yml) 💡

## 🛠️ Build from source

```sh
git clone https://github.com/XDavidT/MongoBoard.git && cd MongoBoard
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # produce MongoBoard.app + .dmg
```

Needs **Node 22+**, **Rust stable** and the **Xcode Command Line Tools** (`xcode-select --install`).

<details>
<summary><b>🧪 Tests</b></summary>

```sh
cd src-tauri
cargo test                                                              # unit tests
MONGODB_URI=mongodb://localhost:27017 cargo test -- --include-ignored   # + CRUD against a real server
```
CI runs both on every push against a `mongo:8` container, then builds the universal macOS app.
</details>

<details>
<summary><b>🧭 Project layout</b></summary>

| Path | What lives there |
|---|---|
| `src-tauri/src/mongo.rs` | All driver calls: connect (SCRAM / AWS IAM), list, find, count, insert / replace / delete |
| `src-tauri/src/shell_json.rs` | Parser + writer for mongosh-style JSON that keeps BSON types intact |
| `src-tauri/src/profiles.rs` | Saved connections (`connections.json` in the app config folder) |
| `src-tauri/src/secrets.rs` | Keychain access |
| `src-tauri/src/commands.rs` | Commands the UI calls + window management |
| `src/views/` | Main window (sidebar, tabs, Quickstart), Connection Manager, Connection editor |
| `src/components/` | Collection tab, tree view, document panel |
</details>

## 🔒 Privacy & security

- No telemetry, no analytics, no accounts. MongoBoard only talks to the databases you connect to.
- Passwords are stored in the macOS Keychain; connection strings are saved **without** credentials.
- Found a vulnerability? Please see [SECURITY.md](SECURITY.md).

## 🤝 Contributing

Bug reports, ideas and pull requests are welcome! Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started.
If MongoBoard saves you time, a ⭐ helps others find it.

## 📄 License

[MIT](LICENSE) © 2026 XDavidT

<sub>MongoDB is a trademark of MongoDB, Inc. Studio 3T and Robo 3T are products of 3T Software Labs; NoSQLBooster is a product of its respective owner. MongoBoard is an independent project, not affiliated with or endorsed by any of them.</sub>
