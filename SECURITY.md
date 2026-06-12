# Security Policy

FingerBrow manages local browser profiles, proxy settings, and Chrome launch arguments. Treat it as software that can affect where browser traffic goes and how profiles are isolated.

## Reporting issues

If this repository is published, please report security issues privately through the repository owner's preferred contact channel before opening a public issue.

## Sensitive data

- Proxy passwords are stored through the host keychain where supported.
- Browser profile data lives in the app data directory.
- Do not share `app.db`, profile folders, logs containing proxy URLs, or screenshots that expose proxy credentials.

## Scope

Security issues include:

- Proxy credentials exposed in logs, UI, files, or command arguments.
- Browser profile isolation failures.
- Launch flags being ignored or silently overridden.
- Unexpected network calls by the app itself.

Out of scope:

- Fingerprinting test-site scoring differences caused by stock Chromium behavior.
- Claims that require OS-level spoofing, such as real MAC address mutation.
- Third-party proxy provider behavior.
