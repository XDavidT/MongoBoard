# MongoBoard — notes for AI agents

Desktop MongoDB client (Tauri 2 + React/TS + official `mongodb` Rust crate). The owner does not write code; agents make all changes. Keep things simple and tested.

## Rules
- One main window (`label: main`, `?view=main`): open connections in the sidebar, collections as tabs, Quickstart tab. The Connection Manager (`?view=manager`) and editor (`?view=editor`) are pop-up windows created in Rust (`commands.rs::focus_or_build`). Connecting goes through `open_connection`, which emits `connections-changed` so the main window refreshes.
- `VITE_DEMO=1` swaps the backend for sample data (`src/demo/`) for screenshots; it is never part of release builds.
- All database access goes through `src-tauri/src/mongo.rs`. The UI never builds BSON: it sends shell-JSON text, Rust parses it with `shell_json.rs`.
- Passwords only in the keychain (`secrets.rs`). Never write secrets to `connections.json` or logs.
- Writes must go through `commands::writable()` (read-only profiles). Production confirmation is in the UI (`DocPanel`).
- Number convention in shell JSON: integer literal → Int32 (or Int64 if large), decimal → Double, `NumberLong(…)` → Int64. Keep the writer/parser round-trip test green.
- Colours are CSS variables in `src/styles.css` with a `prefers-color-scheme: dark` block; don't hard-code colours in components.

## Checks before committing
```sh
npm run build                       # tsc + vite
cd src-tauri && cargo test          # unit tests
MONGODB_URI=… cargo test -- --include-ignored   # CRUD integration (CI runs this with a mongo:8 service)
```

## Releasing
Add a CHANGELOG entry, then bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` on `main`. `release.yml` creates tag `vX.Y.Z` and publishes the `.dmg` automatically (it skips versions that already have a release).
- Auto-update: `tauri-plugin-updater` reads `releases/latest/download/latest.json`; `release.yml` signs it with the `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)` secrets. Never change `plugins.updater.pubkey` without a new key pair, or installed apps stop accepting updates. UI is `src/components/Updater.tsx`.
