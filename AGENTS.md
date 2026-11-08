# AGENTS.md — Valhalla

Theme-aware wallpaper switcher for Linux desktops. Rust + GTK4/libadwaita, Damask-inspired.

## Environment

- Develop inside the `valhalla` distrobox container (Fedora): `distrobox enter valhalla`
- Run all cargo/git commands in the container; the repo is shared with the host.

## Commands

- `cargo build` / `cargo test` / `cargo fmt` / `cargo clippy --all-targets` — clippy and fmt must be clean before committing.
- GUI: `cargo run` · Headless: `cargo run -- --daemon`
- Commits: small and meaningful, conventional style (`feat:`, `chore:`). Never push unless explicitly asked.
- Work in a `git worktree` when the main checkout is occupied (the owner develops in parallel): `git worktree add ../valhalla-<topic> -b <branch>`. Never hold the primary checkout hostage to one task.

## Repo hygiene

- NEVER commit images to the repository except these: app/brand icons under `data/icons/` (build assets, the README logo is the app icon) and the two README showcase screenshots `docs/screenshots/dark.webp` + `docs/screenshots/light.webp` (lossy WebP, q80 — keep them small; the AppStream metainfo points at the PNGs pinned under the immutable `v1.6.2` tag instead). No PR proof images, no screenshot iterations, no image test fixtures (generate or inline them in test code), no asset branches — PR proof screenshots go into the PR description, never into git. Every committed image stays in every clone forever: published releases are immutable, so their tags pin old history and it can never be purged.

## Product rules

- The desktop light/dark preference is law: the wallpaper tone must always match the current scheme, and selection re-runs immediately on theme change.
- Orientation, resolution and aspect ratio are always derived from the monitor — never expose them as user settings.
- If a wallpaper is smaller than the screen, upscale it to cover the screen.
- API keys live in the GNOME Keyring, never in plain config files.
- UI follows GNOME HIG and stays Damask-simple (libadwaita, Blueprint templates compiled in build.rs).
- Refresh must give visible UI feedback (spinner) and return gracefully on timeout or failure — keep the current wallpaper, never strand the spinner.
- Startup refresh is off by default.
- Background operation: "run in background" hides the window and keeps the process; "start at login" manages the `valhalla.service` systemd user unit.

## Gotchas

- gtk4-rs/glib-rs APIs drift between versions: verify signatures against vendored sources in `~/.cargo/registry` before writing bindings code.
- glib objects are not `Send`; settings access from tokio workers goes through the `SendSettings` wrapper.
- Cross-process refreshes are deduplicated by a flock + `last_refresh_at` in the state file — don't bypass it.
- Smoke tests intentionally change the user's real wallpaper; that is expected.
