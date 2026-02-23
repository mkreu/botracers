# BotRacers VSCode Extension

Integrated workflow for BotRacers onboarding, bot bootstrap, build/upload, and artifact management.

## Commands

- `BotRacers: Configure Server URL`
- `BotRacers: Login` (webview form)
- `BotRacers: Initialize Bot Project`
- `BotRacers: Open Bot Project`

## Server URL Profiles

The extension no longer uses a raw `serverUrl` setting.

- `production` (default): `https://racers.mlkr.eu`
- `localhost`: `http://127.0.0.1:8787`
- `custom`: uses `botracers.customServerUrl`

Use `BotRacers: Configure Server URL` to switch profiles.

## Explorer View States

The `BotRacers` tree appears inside the built-in Explorer sidebar and has three explicit states:

- `loggedOut`
  - Renders a VS Code Welcome View with state-specific guidance and actions.
  - Variants: base login prompt, session-expired prompt, generic request-error prompt.
- `needsWorkspace`
  - Renders a VS Code Welcome View with workspace-specific guidance and actions.
  - Variants: missing workspace/Cargo.toml, no discovered binaries.
- `ready`
  - Shows `Local Binaries` and `Remote Artifacts`.

## Explorer Workflow (`ready` state)

- `Local Binaries`
  - Discovers binaries from `Cargo.toml` (`[[bin]]`, including explicit `path`) and `src/bin/*.rs`.
  - Inline icon actions: `Build & Upload`, `Build Binary`, `Reveal ELF Path`.
- `Remote Artifacts`
  - Lists artifacts from `GET /api/v1/artifacts`.
  - Owned artifacts inline icon actions: `Replace`, `Toggle Visibility`, `Delete`.
  - The same owned-artifact actions are also available in the right-click menu.

Replace semantics:
- Upload new build first.
- Delete old artifact after upload (best effort).
- If delete fails, new artifact is kept.

## Bootstrap Template

Template files include:
- `Cargo.toml`
- `.cargo/config.toml`
- `link.x`
- `src/bin/car.rs`

Template behavior:
- Pulls `botracers-bot-sdk` from git (`branch = "main"`).
- Uses SDK defaults for panic handler and global allocator.
- Keeps target/linker wiring local and explicit via `.cargo/config.toml` and `link.x`.

Override points:
- Provide your own panic handler by disabling sdk feature `panic-handler`.
- Provide your own allocator by disabling sdk features `global-allocator` and `allocator-4k`.

## Auth Behavior

- If server reports `auth_required=true`, the extension requires webview login and uses bearer token auth.
- If server reports `auth_required=false` (standalone mode), artifact operations work without login.
