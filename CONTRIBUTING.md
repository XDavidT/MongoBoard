# 🤝 Contributing to MongoBoard

Thanks for helping! Bug reports, ideas, docs fixes and pull requests are all welcome.

## 🐛 Reporting bugs / 💡 suggesting features
Use the [issue templates](https://github.com/XDavidT/MongoBoard/issues/new/choose). For bugs, include your macOS version, MongoBoard version, MongoDB version/hosting (Atlas, self-hosted…) and the exact error text. **Never paste passwords or full connection strings with credentials.**

## 🛠️ Development setup
```sh
npm install
npm run tauri dev
```
Requirements: Node 22+, Rust stable, Xcode Command Line Tools.

## ✅ Before opening a PR
```sh
npm run build                     # TypeScript + Vite
cd src-tauri && cargo test        # Rust unit tests
MONGODB_URI=mongodb://localhost:27017 cargo test -- --include-ignored   # optional: CRUD against a local server
cargo fmt --check
```
CI runs the same checks (with a real `mongo:8` container) on every pull request.

## 🧭 Ground rules
- All database access goes through `src-tauri/src/mongo.rs`; the UI sends shell-JSON text and Rust parses it (`shell_json.rs`).
- Secrets only in the Keychain (`secrets.rs`), never in files or logs.
- Writes must respect read-only connections (`commands::writable`).
- Colours come from CSS variables in `src/styles.css` (light + dark); don't hard-code colours.
- Keep PRs focused; add or update tests for behaviour changes.

## 🚀 Releasing (maintainers)
1. Add a `CHANGELOG.md` entry for the new version.
2. Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, and merge to `main`.
3. The **Release** workflow notices the new version, creates the `vX.Y.Z` tag, builds the universal `.dmg` and publishes the release (with notes from the changelog). Pushing a `vX.Y.Z` tag yourself works too.
4. Automatic updates need two repository secrets: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (the key that matches `plugins.updater.pubkey` in `tauri.conf.json`). Without them the release is still published, but installed apps won't be offered it.
