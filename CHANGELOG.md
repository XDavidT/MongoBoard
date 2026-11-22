# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.3] - 2026-11-22

### Added
- 🗑️ **Drop database** from the sidebar (hover a database → trash icon). You confirm by typing the database name; Production connections show an extra warning. Blocked on read-only connections, and `admin`, `local` and `config` can never be dropped.

## [0.1.2] - 2026-11-22

### Added
- 📋 **Paste a full connection string** into the connection editor (including Studio 3T and Compass exports): the username, password, auth database, mechanism, AWS IAM settings and connection name are filled into their fields, the password goes to the Keychain, Studio 3T-only `3t.*` settings are removed, and the environment tag is guessed from the name.

## [0.1.1] - 2026-11-19

### Changed
- ✏️ Renamed the app from **MongBoard** to **MongoBoard** (repository: XDavidT/MongoBoard). Saved connections and Keychain passwords from 0.1.0 are moved over automatically on first launch.

## [0.1.0] - 2026-11-19

### Added
- 🪟 Main window with an open-connections sidebar (connection → databases → collections), collection tabs and a Quickstart tab with recent connections
- 🗂️ Connection Manager and connection editor as pop-up windows
- 🔑 Username/password (SCRAM) auth with passwords in the macOS Keychain
- ☁️ AWS IAM (`MONGODB-AWS`) auth via the default credential chain or a named AWS profile
- 🏷️ Environment tags, read-only connections and a Production write confirmation
- 🌲 Tree and JSON views, filter / projection / sort / limit and paging
- ✏️ Add, edit, duplicate and delete single documents with a type-preserving shell-JSON editor
- 🌗 Light and dark mode following macOS
- 📦 Universal macOS build (Apple Silicon + Intel) published on GitHub Releases

[0.1.3]: https://github.com/XDavidT/MongoBoard/releases/tag/v0.1.3
[0.1.2]: https://github.com/XDavidT/MongoBoard/releases/tag/v0.1.2
[0.1.1]: https://github.com/XDavidT/MongoBoard/releases/tag/v0.1.1
[0.1.0]: https://github.com/XDavidT/MongoBoard/releases/tag/v0.1.0
