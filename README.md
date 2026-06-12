# FingerBrow

FingerBrow is a local, open-source browser profile launcher. Think of it as a small local replacement for tools like Multilogin, GoLogin, AdsPower, Dolphin Anty, and similar anti-detect/browser-profile managers, but much less ambitious.

It does just enough to make separate Chrome/Chromium profiles look and behave differently: proxy, user-agent, language, timezone, WebRTC policy, window size, startup URLs, Chrome profile color, and app-level spoof identity fields.

## Read This First

No bullshit:

- This is not a production-grade anti-detect browser.
- This is not a guarantee of anonymity.
- This is not magic fingerprint invisibility.
- This is not ready for sensitive accounts, regulated work, secrets, or high-risk operational use.
- This does not rewrite every browser fingerprint surface.
- This does not change your real network-interface MAC address.

This was 100% vibe-coded with Codex. I tried to cover my ass with fairly rigorous testing, local builds, Docker builds, Rust unit tests, frontend linting, and manual browser checks, but you should still treat it as early software.

Use it for local profile management, testing, QA, research, and low-stakes browser separation. Do not trust it with anything you cannot afford to lose.

## Privacy

FingerBrow is local-first. There is no hosted FingerBrow service and no telemetry endpoint in this app.

We do not collect your data.

Your profile metadata, browser user-data directories, proxy settings, and app database live on your machine. Proxy passwords are stored in the host keychain when possible.

That said, your browser traffic still goes wherever you send it. Your proxy provider, websites, browser extensions, DNS setup, operating system, and Chrome itself can still leak or expose data. FingerBrow does not make those risks disappear.

## What It Does

- Creates isolated Chrome/Chromium user-data directories per profile.
- Launches profiles with per-profile proxy settings.
- Supports saved proxy profiles.
- Supports authenticated SOCKS/HTTP proxies through a local relay.
- Stores proxy profile passwords in the host keychain when possible.
- Applies profile-level user-agent, language, timezone, WebRTC, window size, startup URL, and Chrome color settings.
- Adds editable app-level spoof MAC identity fields per profile.
- Can randomize the spoof MAC field on launch.
- Shows which profiles are currently open.
- Keeps app data local on your machine.

## What It Does Not Do

FingerBrow does not provide a custom Chromium engine. It launches your local Chrome/Chromium with flags, preferences, isolated data dirs, and local proxy helpers.

Current limits:

- It does not spoof real hardware MAC addresses at the OS/network layer.
- It does not patch Chromium internals.
- It does not guarantee that fingerprint test sites will agree with each other.
- It does not promise to bypass bot detection, KYC checks, fraud systems, or platform enforcement.
- It does not encrypt the whole browser profile directory.
- It does not sanitize risky extensions you install yourself.

## Platform Support

| Platform | Status           | Notes                                                               |
| -------- | ---------------- | ------------------------------------------------------------------- |
| macOS    | Supported        | Main tested path. Builds `.app` and `.dmg` bundles through Tauri.   |
| Linux    | Supported        | Builds AppImage and Debian packages through native Linux or Docker. |
| Windows  | Not targeted yet | Maybe later. Current docs and build workflow target macOS/Linux.    |

## Requirements

For local development:

- Node.js 22+
- pnpm 10+
- Rust stable
- Chrome, Chromium, or a compatible Chromium binary

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

Typical artifacts:

- `macos/FingerBrow.app`
- `dmg/FingerBrow_0.1.0_aarch64.dmg`

### Linux

Install Linux prerequisites, then run:

```sh
pnpm install
pnpm desktop:build:linux
```

Typical artifacts:

- `appimage/FingerBrow_0.1.0_aarch64.AppImage`
- `deb/FingerBrow_0.1.0_arm64.deb`

## Quick Start: Docker Linux Builder

Docker is provided as a Linux package builder. It does not run the desktop GUI inside the container; it builds Linux artifacts and copies the final packages to `./release`.

```sh
pnpm docker:linux-build
```

Or directly:

```sh
docker compose run --rm --build linux-builder
```

After the build finishes:

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

FingerBrow stores profile metadata, Chrome user-data directories, and local settings in the platform app data directory.

The current development identifier is intentionally kept as:

```text
com.xiaochi.local-chromium-manager
```

That preserves existing local profiles created during the early FingerBrow development cycle. A future breaking release can migrate this identifier if needed.

Do not commit app data, browser user-data folders, proxy credentials, generated databases, or profile exports containing secrets.

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
- Saved proxy or manual proxy
- Browser/OS user-agent preset
- Language
- Timezone
- WebRTC policy
- Window size and position
- Startup URLs
- Extra launch arguments
- Chrome profile color
- App-level spoof MAC field
- Randomize spoof MAC field on launch

## Testing Done So Far

Current checks used during development:

```sh
pnpm format:check
pnpm lint
pnpm build
cd src-tauri && cargo test
pnpm desktop:build:mac
pnpm docker:linux-build
```

Also manually tested:

- Chrome launches with isolated profiles.
- Authenticated SOCKS proxy routing through the local relay.
- Proxy IP checks against public IP pages.
- Profile color writing into Chrome preferences.
- Running-profile detection and close button.
- Default macOS launcher layout and drag region.

None of this means it is production-safe. It means it has been kicked pretty hard for a 0.1.0 local tool.

## Useful Commands

```sh
pnpm desktop:dev         # run the Tauri app in development
pnpm desktop:build       # build desktop bundle for current platform
pnpm desktop:build:mac   # build macOS .app/.dmg artifacts
pnpm desktop:build:linux # build Linux AppImage/deb artifacts
pnpm docker:linux-build  # build Linux artifacts with Docker Compose
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
