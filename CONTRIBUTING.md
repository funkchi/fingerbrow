# Contributing to FingerBrow

Thanks for helping improve FingerBrow. The project is intentionally local-first and conservative: changes should keep profile data on the user's machine, avoid hidden network calls, and make browser launch behavior explicit.

## Local setup

```sh
pnpm install
pnpm desktop:dev
```

## Before opening a pull request

Run:

```sh
pnpm format:check
pnpm lint
pnpm build
cd src-tauri && cargo test
```

## Development guidelines

- Keep profile data local unless a feature clearly requires otherwise and the user opts in.
- Prefer explicit launch flags and stored profile settings over hidden browser mutations.
- Do not commit proxy credentials, profile databases, or generated browser profile folders.
- Keep UI changes dense and operational; FingerBrow is a work tool, not a landing page.
- Document any fingerprinting limitation honestly. FingerBrow can shape profile settings; it is not a magic anonymity guarantee.
