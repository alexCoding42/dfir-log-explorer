# Desktop build verification

The active workflow is [mvp-2.yml](../../.github/workflows/mvp-2.yml). It runs on pull requests and integration-branch pushes, with read-only repository permissions. Do not merge the pull request without human approval.

The MVP 1 workflow remains manually dispatchable for historical revisions.

The interface job checks TypeScript, the production asset bundle, formatting, preview boundaries, and Chromium workflows. The desktop matrix builds on Ubuntu 24.04 and Windows 2025, runs workspace Rust tests and Clippy in release mode, and packages Linux `.deb` and Windows NSIS installers. An explicit host target keeps Windows's static runtime flags away from build-script and procedural-macro host binaries.

The workflow uses synthetic data only and retains platform build artifacts for 14 days. No credentials or private evidence are required or published. Build infrastructure can download public dependencies; application runtime and log processing remain local.

Rust dependency artifacts are cached after a successful job, per platform/toolchain to avoid recompiling the embedded C++ engine during subsequent MVP changes. Workspace sources are rebuilt and checked after changes.

Open a successful workflow run and download `dfir-log-explorer-linux` or `dfir-log-explorer-windows` from its artifacts section. Extract the archive and follow the [MVP 2 acceptance guide](../mvp-2-acceptance.md). Files are staged in a single directory before upload, so the ZIP root contains only:

- Linux: `DFIR Log Explorer_0.3.0_amd64.deb` and `dfir-log-explorer`.
- Windows: `DFIR Log Explorer_0.3.0_x64-setup.exe`.

Staging fails on missing/unexpected files or nested directories. Prefer installing the Linux `.deb`; GitHub artifact ZIPs do not retain executable permissions for the standalone Linux binary. Platform build success does not substitute for interactive Windows/Linux testing or an offline runtime audit.

See [MVP 2 validation](../mvp-2-validation.md) for current checks and [MVP 0 validation](../validation.md) for the original measurements. Local Linux packages use the debug profile for native acceptance checks; workflow installers use the release profile.

## Windows user installation

NSIS explicitly uses `currentUser`, with an `asInvoker` installer manifest. A fresh installation proposes `%LOCALAPPDATA%\Programs\DFIR Log Explorer`; shortcuts and uninstall registration belong to that user. Existing install locations and explicit `/D=` destinations are retained. A small hook adjusts only the fresh-install default in interactive and silent mode; the upstream Tauri template remains in use.

The Windows build executes `tests/windows_install.ps1` against the actual installer: manifest, default location, explicit destination, reinstall, current-user Start menu shortcut/registry and uninstall. CI uses its existing runner account; an interactive test with a standard Windows account is still required. If WebView2 is missing, the embedded offline runtime installer is used.
