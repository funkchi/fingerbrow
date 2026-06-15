# FingerBrow

FingerBrow is an open-source, self-hosted browser fingerprint profile manager. It launches isolated local Chromium/Chrome profiles with explicit proxy, user-agent, language, timezone, WebRTC, startup URL, window, and profile color settings.

It is designed for people who want a local desktop tool for building and testing distinct browser profiles without sending profile configuration to a hosted SaaS.

## What It Does

- Creates isolated Chrome user-data directories per profile.
- Can install an app-managed Chrome for Testing binary under FingerBrow app data.
- Launches profiles with per-profile proxy settings.
- Supports authenticated SOCKS/HTTP proxies through a local relay.
- Stores proxy profile passwords in the host keychain when possible.
- Applies profile-level user-agent, language, timezone, WebRTC, window size, startup URL, and Chrome color settings.
- Shows which profiles are currently open.
- Keeps app data local on your machine.

## What It Is Not

FingerBrow is not a guarantee of anonymity and does not magically rewrite every browser fingerprint surface. Stock Chromium still controls many fingerprinting behaviors. FingerBrow helps make profile configuration explicit, repeatable, and locally managed.

FingerBrow does not spoof real hardware MAC addresses per browser profile. That kind of change belongs at the OS/network layer.

## Platform Support

| Platform | Status           | Notes                                                                        |
| -------- | ---------------- | ---------------------------------------------------------------------------- |
| macOS    | Supported        | Best current path. Builds `.app` and `.dmg` bundles through Tauri.           |
| Linux    | Supported        | Builds AppImage and Debian packages through native Linux or Docker.          |
| Windows  | Not targeted yet | The app may build later, but this repo currently documents macOS/Linux only. |

## Requirements

For local development:

- Node.js 22+
- pnpm 10+
- Rust stable
- Chrome, Chromium, or a compatible Chromium binary

FingerBrow can also install a managed Chrome for Testing binary from the Settings tab. When installed, that binary is preferred by default so profiles are not launched through your normal `/Applications/Google Chrome.app`.

For Linux desktop builds, install the Tauri/WebKit dependencies listed in the Tauri Linux prerequisites, or use the Docker builder below.

## Quick Start: Run From Source

```sh
git clone <your-fork-url> fingerbrow
cd fingerbrow
pnpm install
pnpm desktop:dev
```

The development app opens a Tauri desktop window and uses your local app data directory for profiles.

## Build It Yourself

### macOS

```sh
pnpm install
pnpm desktop:build:mac
```

Outputs are written under:

```text
src-tauri/target/release/bundle/
```

Typical artifacts include:

- `macos/FingerBrow.app`
- `dmg/FingerBrow_*.dmg`

### Linux

Install Linux prerequisites, then run:

```sh
pnpm install
pnpm desktop:build:linux
```

Typical artifacts include:

- `appimage/FingerBrow_*.AppImage`
- `deb/fingerbrow_*.deb`

## Quick Start: Docker Linux Builder

Docker is provided as a reproducible Linux package builder. It does not run the desktop GUI inside the container; it builds Linux artifacts and copies them to `./release`.

```sh
docker compose run --rm linux-builder
```

After the build finishes, check:

```sh
ls -R release
```

Use the AppImage or Debian package on a Linux desktop with a compatible Chromium/Chrome installation.

## Install on macOS

Build the macOS package:

```sh
pnpm desktop:build:mac
```

Then either:

- Open the generated `.dmg` and drag `FingerBrow.app` into `/Applications`.
- Or copy the `.app` directly:

```sh
cp -R src-tauri/target/release/bundle/macos/FingerBrow.app /Applications/
```

If macOS blocks an unsigned local build, open it from Finder once with Control-click, then choose Open.

## Data Storage

FingerBrow stores profile metadata, Chrome user-data directories, and local settings in the platform app data directory. The current development identifier is intentionally kept as:

```text
com.xiaochi.local-chromium-manager
```

That preserves existing local profiles created during the early FingerBrow development cycle. A future breaking release can migrate this identifier if needed.

Do not commit app data, browser user-data folders, proxy credentials, or generated databases.

## Proxy Profiles

FingerBrow supports saved proxy profiles and per-browser-profile proxy assignment.

Supported proxy types:

- SOCKS5
- SOCKS4
- HTTP
- HTTPS

For authenticated SOCKS proxies, FingerBrow starts a local relay and launches Chrome through `127.0.0.1:<relay-port>` so Chrome traffic still reaches the authenticated upstream proxy.

## Browser Profile Controls

Profile settings currently include:

- Browser binary path
- App-managed Chrome for Testing install/status
- Saved proxy or manual proxy
- Browser/OS user-agent preset
- Language
- Timezone
- WebRTC policy
- Window size and position
- Startup URLs
- Extra launch arguments
- Chrome profile color

## Useful Commands

```sh
pnpm desktop:dev        # run the Tauri app in development
pnpm desktop:build      # build desktop bundle for current platform
pnpm desktop:build:mac  # build macOS .app/.dmg artifacts
pnpm desktop:build:linux # build Linux AppImage/deb artifacts
pnpm docker:linux-build # build Linux artifacts with Docker Compose
pnpm format:check
pnpm lint
pnpm build
cd src-tauri && cargo test
```

## Continuous Builds

The repository includes a GitHub Actions workflow at `.github/workflows/build.yml` for:

- formatting, linting, frontend build, and Rust tests
- macOS `.app` and `.dmg` packaging
- Linux AppImage and Debian packaging

## Repository Layout

```text
src/                 React UI
src-tauri/           Tauri/Rust backend
src-tauri/icons/     App bundle icons
.github/workflows/   CI checks and macOS/Linux package builds
Dockerfile           Linux desktop package builder
docker-compose.yml   One-command Docker build wrapper
release/             Docker build output, ignored by git
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).
