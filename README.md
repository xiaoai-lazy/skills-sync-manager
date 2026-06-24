# Skills Sync Manager

A cross-platform desktop app scaffold for managing one main skills directory and syncing selected skills into target directories.

## Tech stack

- Tauri 2
- React
- TypeScript
- Vite
- Rust

## Scripts

- `npm run dev` starts the Vite frontend dev server.
- `npm run build` type-checks and builds the frontend.
- `npm run test` runs frontend tests with Vitest.
- `npm run tauri` runs the Tauri CLI.
- `npm run tauri:dev` starts the Tauri desktop app in development mode.

## Backend checks

Run Rust checks from the Tauri crate directory:

```bash
cd src-tauri
cargo test
```

`Cargo.lock` should be generated and committed by the first development environment with Rust/Cargo available.
