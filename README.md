# FingerBrow

FingerBrow is a free, open-source, local browser profile launcher.

It is meant to be a small, hackable alternative to tools like Multilogin, GoLogin, AdsPower, and Dolphin Anty / dolphin-anti. The goal is not to be magical. The goal is to do enough local browser-profile juggling to make Chrome profiles look different from each other: proxy, user-agent, language, timezone, WebRTC behavior, profile color, startup URLs, and isolated user-data folders.

Also: this thing is 100% vibe coded with Codex. I tried to cover my ass with formatting, linting, Rust tests, release builds, and real browser checks. Still, treat it like a sharp little homemade tool, not enterprise security software.

## Read This First

- Not safe for production.
- Not safe for sensitive data.
- Not a promise of anonymity.
- Not a silver bullet against fingerprinting.
- Not a replacement for understanding what your proxy, browser, OS, and websites are doing.
- Does not collect your data.
- Does not phone home with your profiles.
- Stores app/profile data locally on your machine.

If you need something for high-stakes accounts, money, customer data, or anything that would make your stomach hurt if it broke, do not trust this yet.

## Platforms

FingerBrow targets:

- macOS
- Linux

Windows is not the target right now.

On macOS, the easiest path is to build it yourself. The app is unsigned, so downloading a random `.app` or `.dmg` will make macOS complain. Building locally avoids most of that Gatekeeper drama because the app came from your own machine.

## What It Does

- Creates one isolated browser user-data folder per profile.
- Can install and use an app-managed Chrome for Testing, separate from your normal Chrome.
- Launches profiles with saved proxy settings.
- Supports SOCKS4, SOCKS5, HTTP, and HTTPS proxies.
- Supports authenticated SOCKS proxies through a local relay.
- Stores proxy passwords in the host keychain when possible.
- Applies user-agent, language, timezone, WebRTC, window, startup URL, and Chrome color settings.
- Shows which profiles are running.
- Keeps profile config local.

## What It Does Not Do

FingerBrow does not fully rewrite every browser fingerprint surface. Stock Chromium still leaks plenty of truth if a site asks the right questions.

It also does not change your real network hardware MAC address per browser profile. Browser-level MAC spoofing is mostly theater; real MAC changes belong at the OS/network layer.

## Requirements

- Node.js 22+
- pnpm 10+
- Rust stable
- macOS or Linux
- Chrome/Chromium, or the managed Chrome for Testing installed from the app Settings tab

## Run From Source

```sh
git clone <your-fork-url> fingerbrow
cd fingerbrow
pnpm install
pnpm desktop:dev
```

## Build It Yourself

### macOS

```sh
pnpm install
pnpm desktop:build:mac
```

Build output:

```text
src-tauri/target/release/bundle/macos/FingerBrow.app
src-tauri/target/release/bundle/dmg/FingerBrow_*.dmg
```

Install however you like:

```sh
cp -R src-tauri/target/release/bundle/macos/FingerBrow.app /Applications/
```

If macOS still blocks it, Control-click the app in Finder and choose Open once.

### Linux

Install the normal Tauri/WebKit Linux prerequisites, then:

```sh
pnpm install
pnpm desktop:build:linux
```

Typical output:

```text
src-tauri/target/release/bundle/appimage/FingerBrow_*.AppImage
src-tauri/target/release/bundle/deb/fingerbrow_*.deb
```

There is also a Docker builder for Linux packages:

```sh
docker compose run --rm linux-builder
```

Docker build output goes to:

```text
release/
```

## Proxy Notes

FingerBrow supports saved proxy profiles and per-browser-profile proxy assignment.

Supported proxy types:

- SOCKS5
- SOCKS4
- HTTP
- HTTPS

For authenticated SOCKS proxies, FingerBrow starts a local relay and points Chrome at `127.0.0.1:<relay-port>`. The relay handles upstream authentication.

## Browser Profile Controls

Profile settings currently include:

- Browser binary path
- Managed Chrome for Testing install/status
- Saved proxy or manual proxy
- Browser/OS user-agent preset
- Language
- Timezone
- WebRTC policy
- Window size and position
- Startup URLs
- Extra launch arguments
- Chrome profile color

## Data Storage

FingerBrow stores profile metadata, browser user-data folders, and local settings in your platform app data directory.

The current app identifier is still:

```text
com.xiaochi.local-chromium-manager
```

That weird old name is intentional for now, because it preserves existing local profiles from the early development phase. A later breaking release can clean it up.

Do not commit app data, generated browser profiles, proxy credentials, or local databases.

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

## CI

The GitHub Actions workflow checks formatting, linting, frontend build, Rust tests, and macOS/Linux package builds.

## Repository Layout

```text
src/                 React UI
src-tauri/           Tauri/Rust backend
src-tauri/icons/     App bundle icons
.github/workflows/   CI checks and package builds
Dockerfile           Linux desktop package builder
docker-compose.yml   Docker build wrapper
release/             Docker build output, ignored by git
```

## License

MIT. Free as in "go break it in your own interesting way."
