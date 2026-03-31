# ColdVox GUI

ColdVox now uses a **Tauri v2 + React** overlay shell under this folder.

This tranche replaces the old Qt/QML placeholder with a Windows-first transparent host shell that is intentionally narrow in scope:

- collapsed idle presence
- expanded transcript panel
- visible state feedback (`idle`, `listening`, `processing`, `ready`, `error`)
- clear separation between live partial text and committed final text
- typed Tauri command/event seam exercised by a demo driver

It is **not** a Mini lift-and-shift and it does **not** wire real STT, injection, hotkeys, or settings persistence yet.

## Layout

```text
crates/coldvox-gui/
├── src/                    # React frontend
│   ├── components/
│   ├── contracts/
│   ├── hooks/
│   └── lib/
└── src-tauri/              # Rust Tauri host shell package
    └── src/
```

## Key Entry Points

- Frontend shell: [`src/App.tsx`](./src/App.tsx)
- Frontend contract hook: [`src/hooks/useOverlayShell.ts`](./src/hooks/useOverlayShell.ts)
- Rust host shell: [`src-tauri/src/lib.rs`](./src-tauri/src/lib.rs)
- Rust state model: [`src-tauri/src/state.rs`](./src-tauri/src/state.rs)

## Development Commands

Run these from `crates/coldvox-gui/`:

```bash
npm install
npm run test
npm run build
npm run tauri dev
```

Rust verification still happens through the workspace package:

```bash
cargo check -p coldvox-gui
cargo test -p coldvox-gui
```

## Current Runtime Reality

- The frontend renders a restrained overlay shell, not the final product UI.
- The Rust side owns window sizing/bootstrap plus demo command/event emission.
- The demo driver proves the UI contract end-to-end without touching the real audio/STT runtime.
