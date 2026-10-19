# DFIR Log Explorer

**Spend less time preparing logs. Spend more time investigating.**

A focused log exploration tool for digital forensic and incident response workflows. DFIR Log Explorer helps analysts turn collected log files into structured, searchable events, with the original log content and parsing diagnostics available alongside the results.

**Project status:** `main` contains Delivery 0 and the integrated **Delivery 1 (v0.2.1)**, including the compact workspace and Statistics improvements. **Delivery 2 (v0.3.0)** is proposed in [PR #6](https://github.com/a-cisse/dfir-log-explorer/pull/6), awaiting human acceptance and integration. These are early test builds, not a production release. “Delivery” replaces the historical MVP planning label; existing file paths, branch names and application versions remain unchanged.

## Overview

Investigations often start with raw logs collected from compromised or potentially compromised systems. Before an analyst can follow an IP address, examine a request or compare timestamps, those files need to become understandable datasets.

DFIR Log Explorer brings preconfigured parsing, filtering, event inspection and visualization into a local desktop workspace for Windows and Linux. Its purpose is to reduce repetitive preparation work so analysts can spend more time examining what happened. It uses Rust and an embedded DuckDB engine behind a Tauri 2 interface built with React and TypeScript.

The application focuses on examining collected files. It fits alongside scripts, centralized log platforms and specialized forensic tools; its scope is interactive log exploration rather than continuous ingestion, monitoring or security alerting.

## The problem

Preparing logs can involve reviewing raw text, combining `grep`, `awk` or `jq` commands, writing Python scripts, building parsing expressions, converting results to CSV and moving them into spreadsheets or visualization tools. Different formats and timestamp conventions add work, and transformations can make parsing failures or the original event harder to inspect.

These approaches remain useful for their own workflows. DFIR Log Explorer offers a consistent interface for the supported formats, bringing field extraction and exploratory analysis together without requiring a separately managed log platform.

## The solution

**Import → Parse → Explore → Investigate**

1. **Import:** Select a collected log file through the desktop file picker. The source is opened read-only, with progress and cancellation available.
2. **Parse:** Use a preconfigured reader or automatic recognition. Review extraction results, parsing status and warnings.
3. **Explore:** Search raw content and fields, combine filters, sort results and inspect event distributions.
4. **Investigate:** Follow relevant events while checking their raw content, source positions and diagnostics. The analyst remains responsible for interpreting the evidence.

## Why DFIR Log Explorer?

- **Less repetitive preparation:** Built-in readers extract supported fields without requiring a new script or parsing expression for each supported dataset. Reducing time to investigation is a product objective, not a measured claim of superiority.
- **Integrated exploration:** Read imported events, search and inspect raw content in one workspace. In Delivery 1, optional filters and statistics stay out of the default reading view.
- **Visible parsing quality:** Separate successful interpretation, partial extraction and unrecognized records. Malformed lines can still be examined instead of disappearing from the results.
- **Original content within reach:** Inspect raw text, source line numbers and byte offsets alongside extracted fields. Source hashes support comparison with the imported file.
- **Local setup:** Log analysis uses an embedded database and packaged UI assets. It does not require an Elasticsearch, Kibana, Splunk or other externally managed log backend.

The tool is intended to complement existing workflows, including Graylog, Timesketch, Timeline Explorer and specialized forensic parsers. No comparative benchmark establishes that it is faster or more capable than those tools. See the [performance notes](docs/performance.md) and [validation record](docs/validation.md) for measured workloads and their limits.

## Features

### Foundation from Delivery 0

- Import one uncompressed text source into a temporary analysis session, with automatic Apache recognition or explicit Apache/plain-text selection.
- Search raw logs and extracted fields using case-insensitive literal substrings. Combine parsing status, warnings, exact-field and explicit UTC time filters.
- Sort across the complete filtered dataset and browse virtualized pages of at most 500 events. Configure visible columns.
- Inspect complete extracted HTTP fields, raw events, source line numbers, byte offsets and parsing diagnostics.
- Review parsing counts, HTTP response statistics and a compact interactive timeline for all matching events, rather than just the visible page.
- Track import progress and cancel an import. Cancellation or failure rolls back the import and preserves the previous dataset.
- Inspect the source's SHA-256 fingerprint; use Light, Dark or System themes, persistent display preferences, compact navigation and keyboard shortcuts.

### Integrated in Delivery 1

- Start with a compact log table immediately after import: line, timestamp, parsing status and raw content. Open additional filters and Statistics when needed; save/open/new actions are grouped in the Case menu.
- Save temporary analysis as a local `.dfir` case and reopen it without access to the original source files. Further imports into a saved case are retained automatically.
- Append multiple sources to a case, with stable source identities, source summaries and the correct provenance in the event inspector. Duplicate imports remain distinct sources.
- Select explicit Nginx access and JSON Lines readers, retaining original JSON fields and independent validation diagnostics.
- Combine source, identified-reader, timestamp-presence and exact JSON scalar filters with the existing filters. Compare timestamps across sources using UTC instants while retaining source offsets.

The [Delivery 1 acceptance guide][mvp1-acceptance], [reader/date rules][mvp1-formats] and [validation record][mvp1-validation] describe that implementation and its checks. A custom parser editor is not currently available; the application uses preconfigured readers.

### Understanding parsing results

| Status                         | Meaning                                                                                                 |
| ------------------------------ | ------------------------------------------------------------------------------------------------------- |
| **Parsed**                     | The reader accepted the record without field-validation errors.                                         |
| **Partial — partially parsed** | Some fields were recovered, but others failed validation. Recoverable fields remain available.          |
| **Unparsed**                   | The record could not be interpreted as a supported structured event. Its raw content remains available. |

Warnings are counted independently of parsing status. For example, a text record can be Parsed while carrying an encoding warning. Explicit plain-text mode marks successfully read lines Parsed without claiming structured extraction. Missing optional fields do not automatically mean parsing failed, and a Parsed status is not a guarantee of evidential reliability.

## Supported log formats

| Reader        | Availability              | Supported scope                                                                                                                              |
| ------------- | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Apache access | Delivery 0 and Delivery 1 | Standard Common and Combined layouts; not every configurable Apache `LogFormat`.                                                             |
| Plain text    | Delivery 0 and Delivery 1 | Uncompressed text, one retained record per source line, without structured extraction.                                                       |
| Nginx access  | Delivery 1                | Standard Common/Combined access layouts; custom layouts and Nginx error logs are not supported.                                              |
| JSON Lines    | Delivery 1                | One JSON object per line. Known HTTP/date aliases are normalized; original fields, including nested values, remain available for inspection. |

Auto mode recognizes Apache-compatible access layouts; Delivery 1 also recognizes JSON objects. Apache and Nginx Combined layouts overlap, so automatic recognition cannot establish which server produced a record. Choose Nginx explicitly in Delivery 1 to label those events Nginx.

In the JSON Lines reader, malformed JSON, non-object values and blank lines remain Unparsed with diagnostics. Exact JSON filters cover top-level strings, numbers and booleans. See the [reader/date rules][mvp1-formats] for aliases, duplicate-key handling and timestamp validation.

Apache error logs, firewall/WAF/Syslog/HAProxy readers, binary logs and multiline grouping are not currently implemented. Future format support is listed in the [roadmap](#roadmap).

## Screenshots

These captures show the integrated Delivery 1 v0.2.1 build with synthetic evidence in the labeled browser preview. The default workspace prioritizes the logs; statistics and additional filters open on demand.

Compact event exploration with raw content visible:

![Delivery 1 compact log table with source and parsing filters](https://raw.githubusercontent.com/a-cisse/dfir-log-explorer/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/images/mvp-1-minimal-light.png)

Inspecting a partially parsed event alongside the logs:

![Delivery 1 inspector showing the original log line and invalid timestamp diagnostic](https://raw.githubusercontent.com/a-cisse/dfir-log-explorer/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/images/mvp-1-minimal-inspector.png)

See the [minimal workspace report][mvp1-workspace] for layout checks and Dark mode. The [Delivery 0 redesign report](docs/ui-ux-redesign.md) retains the earlier interface captures.

## Getting started

### Download a desktop test build

Initial supported platforms are **Ubuntu 24.04 x64** and **Windows 11 x64**. Successful CI runs provide:

- `dfir-log-explorer-linux`: a `.deb` package and application executable.
- `dfir-log-explorer-windows`: an NSIS setup executable.

Use the [Delivery 1 workflow](https://github.com/a-cisse/dfir-log-explorer/actions/workflows/mvp-1.yml) for the integrated version, or the [Delivery 2 review workflow](https://github.com/a-cisse/dfir-log-explorer/actions/workflows/mvp-2.yml) for the security readers proposed in PR #6. Check the branch and commit shown by the run, download your platform's artifact and extract it. In Delivery 1 v0.2.1, the setup executable (Windows), or package and executable (Linux), appear directly at the ZIP root. Artifacts depend on successful platform builds and are retained for 14 days; build locally if a suitable artifact is unavailable.

On Ubuntu, install the `.deb` using your normal package manager. On Windows, run the setup executable normally. A fresh installation proposes `%LOCALAPPDATA%\Programs\DFIR Log Explorer` for the current user without requesting administrator elevation. Start menu shortcuts and uninstall registration are user-specific. Existing installation directories and explicitly chosen destinations are retained. Windows test packages are unsigned. Follow the [Delivery 0 acceptance guide](docs/mvp-0-acceptance.md) or [Delivery 1 acceptance guide][mvp1-acceptance] for the version you downloaded. See [build verification](docs/ci/README.md) for the integrated workflow and [Delivery 1 build verification][mvp1-builds] for the corrected downloads.

Packaged applications do not require Rust or Node.js runtimes. They use the operating system's webview: WebKitGTK on Linux and WebView2 on Windows. The Windows package embeds the offline WebView2 installer, so installing that runtime does not require an Internet connection. The shared runtime can subsequently be maintained by Windows; bundling a fixed runtime is a future distribution improvement.

### Build locally

Install **Rust 1.99.0**, selected by [rust-toolchain.toml](rust-toolchain.toml), and **Node.js 24 LTS**. Exact dependency resolutions are committed in [Cargo.lock](Cargo.lock) and [package-lock.json](package-lock.json).

On Ubuntu 24.04, install the desktop build prerequisites:

```sh
sudo apt-get install --no-install-recommends build-essential pkg-config libwebkit2gtk-4.1-dev librsvg2-dev patchelf
```

On Windows, install Microsoft C++ Build Tools with the **Desktop development with C++** workload and the Windows SDK. Install WebView2 if unavailable. See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for platform setup.

```sh
git clone https://github.com/a-cisse/dfir-log-explorer.git
cd dfir-log-explorer
```

A fresh clone checks out `main` with Delivery 1 and the integrated Statistics improvements. To reproduce the pinned Delivery 1 validation build instead of the latest `main`, optionally switch to its historical revision before installing dependencies:

```sh
git switch --detach 9684e828fbc9c84ff87a95d7504cb6f81aa934bc
```

Then install dependencies and launch the desktop application:

```sh
npm ci
npm run desktop:dev
```

Build an installer for the checked-out version:

```sh
npm run desktop:build
```

Bundles normally appear under `target/release/bundle/`. On Windows, use `npm run desktop:build -- --target x86_64-pc-windows-msvc` for consistent packaging; output then appears under `target/x86_64-pc-windows-msvc/release/bundle/`. This explicit target keeps static-runtime flags separate from procedural-macro host builds.

### Preview the interface

```sh
npm ci
npm run dev
```

Open `http://127.0.0.1:1420` and choose **Explore a synthetic sample**. Browser mode explores only the bundled synthetic sample. **Real file imports require the desktop application.** The preview has no upload API and does not implement a second production parser.

## Usage

1. Select **Auto detect** or the appropriate preconfigured reader, then choose **Import log file** in the desktop app. For a first walkthrough, use the synthetic sample.
2. Start reading logs and check their Parsed, Partial, Unparsed and warning indicators before relying on extracted fields. Delivery 1 shows raw content by default; add extracted-field columns as needed.
3. Search for an IP address, request path or timestamp. In the sample, `POST /LOGIN` matches two events regardless of case.
4. Combine exact-field, parsing or warning filters with a UTC time range. In Delivery 1, open **Filters** for additional controls, or **Statistics** for aggregates and timeline intervals. Selecting an interval returns to matching logs.
5. Select an event and switch between **Details**, **Raw Log** and **Parse Diagnostics**. Check the original line and byte offset against the source when necessary.
6. In **Delivery 1**, open **Case → Save case as…** to retain the analysis, add further sources with **Import log file**, and select a source or all sources for exploration. Use **Case → Open case…** to resume the saved investigation.

Table/inspector timestamps retain source offsets; timeline intervals and time filters use UTC, with an inclusive start and exclusive end. Missing or invalid timestamps stay outside time ranges and the timeline. Delivery 1 rejects ambiguous JSON dates and does not guess the units of numeric timestamps; see the [date rules][mvp1-formats].

Shortcuts include **Ctrl/Cmd O** to import, **Ctrl/Cmd K** to focus search, arrow keys to navigate visible rows, **Enter** to inspect and **Esc** to close the inspector or help (also the Case menu in Delivery 1).

## Technical limits and local data handling

### Sessions and cases

- **Delivery 0:** One temporary source. A successful new import replaces the previous dataset; normal application exit removes session storage.
- **Delivery 1:** Imports append sources. **Save case as…** retains all imported evidence; saved cases retain later successful imports automatically. Saving does not overwrite existing files. Cases use schema version 1, with no automatic migration of unknown versions.
- Unsaved Delivery 1 cases remain temporary. A populated temporary case must be saved before switching cases through the UI. Saved cases have no source-removal function or saved-search persistence.
- Saved cases contain raw evidence, extracted fields, source paths and hashes without application encryption. Close a case or the application before copying/backing it up because DuckDB can use an adjacent write-ahead log. Crashes or forced termination can leave temporary storage behind; secure erasure is not implemented.

### Import and query boundaries

- Sources are read-only. Accepted imports retain original bytes, including line terminators and invalid UTF-8, alongside decoded display text. Invalid UTF-8 uses replacement characters for display; original bytes remain stored.
- Use stable source files. Size or modification-time changes detected during import cause rollback. Those checks and SHA-256 fingerprints do not constitute forensic acquisition or a complete chain-of-custody system.
- Only uncompressed UTF-8 text is supported. Known gzip, ZIP, EVTX and UTF-16 signatures are rejected. Dedicated binary readers, additional encodings, compressed sources and multiline grouping are future work.
- A line exceeding **1 MiB**, including its terminator, causes an explicit error and full rollback; it is not silently truncated.
- DuckDB's **512 MB buffer-memory budget is not a process-wide RAM limit**. Original bytes and decoded text are both stored, so sufficient local disk space is required.
- Searches scan text/fields and sorting runs in DuckDB. Unfiltered pages use event-ID boundaries; deep filtered offsets can become slower. Full-text indexing is future work.
- List fields are bounded previews, including URLs and user agents. Selecting an event returns its complete extracted fields and raw text; Delivery 1 returns full JSON objects only in event details.
- One import runs at a time. Queries and event details are unavailable during import. Cancellation or failure preserves the previous dataset/sources.

### Local operation

Analysis runs locally without an account, API key or cloud backend. The application has no telemetry, remote UI resources, remote parser downloads or automatic application updater. DuckDB external access and automatic extension downloads are disabled. Dependency downloads during development/building are separate from application runtime.

This does not establish a verified zero-egress environment: operating-system services, antivirus and the shared WebView2 runtime have their own behavior. Interactive Windows acceptance and a fresh native/offline traffic audit were not performed for Delivery 1. Read the [privacy notes](docs/privacy.md) for Delivery 0 and the [Delivery 1 privacy notes][mvp1-privacy] for saved-case handling.

## Roadmap

The project advances through **deliveries**, retaining the former stage numbers 0–5. Each delivery needs human acceptance and the next needs a separate development authorization. There are no promised release dates or automatic merges. [Project 4](https://github.com/users/a-cisse/projects/4) and [Issues](https://github.com/a-cisse/dfir-log-explorer/issues) hold live progress, priorities, estimates and decisions; the [roadmap](docs/roadmap.md) defines scope and historical references.

| Stage      | Scope and tracking                                                                                                                                                                                                                           |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Delivery 0 | Accepted and integrated: single-source exploration, Apache/text readers, parsing diagnostics, raw inspection, filters, sorting, basic statistics and timeline. Historical PRs #1 and #2 are preserved.                                       |
| Delivery 1 | Integrated: persistent cases, multiple sources, Nginx/JSON Lines, structured/date filters and compact workspace. Historical PRs #3, #4 and #5 are preserved.                                                                                 |
| Delivery 2 | In review: Syslog, Netfilter/iptables, HAProxy HTTP and bounded native ModSecurity audit in [PR #6](https://github.com/a-cisse/dfir-log-explorer/pull/6). Remaining acceptance: [#8](https://github.com/a-cisse/dfir-log-explorer/issues/8). |
| Delivery 3 | Backlog: expanded timeline/statistics workflows and saved searches. [Delivery 3 — Exploration and saved searches](https://github.com/a-cisse/dfir-log-explorer/milestone/2). Basic timeline/statistics already exist.                        |
| Delivery 4 | Backlog: local parsing profiles, preview, versioning and profile-based import. [Delivery 4 — Local parsing profiles](https://github.com/a-cisse/dfir-log-explorer/milestone/3).                                                              |
| Delivery 5 | Backlog: measurements, justified optimization, resumable ingestion, compression and exports. [Delivery 5 — Volume, ingestion and exports](https://github.com/a-cisse/dfir-log-explorer/milestone/4).                                         |
| To explore | Uncommitted: binary readers, correlations and annotations. [To explore — Future work to agree](https://github.com/a-cisse/dfir-log-explorer/milestone/5).                                                                                    |

Future feature Issues remain in Backlog until authorized. Necessary performance work can occur within an earlier authorized delivery. Existing `mvp-*` documentation paths, workflow IDs and branch names remain valid historical references; no retrospective development tickets have been manufactured.

## Contributing

Report issues with the application build/commit, operating system, source format and a reproducible **synthetic** example. Reader changes should include fixtures and tests that verify extraction, timestamp handling, diagnostics and access to original content. Propose changes on a feature branch through a pull request targeting `main`.

Keep private evidence out of issues, commits and build artifacts. The repository ignores agent instruction files, private configuration, credentials/environment overrides, private case/data/export directories and build output. Only synthetic examples under `tests/fixtures/` are intended for version control. `.gitignore` does not remove files already tracked or published in history; review every commit for sensitive content.

### Development checks

Run these commands from the repository root:

```sh
npm run build
cargo fmt --all --check
cargo test -p dfir-log-core --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm test
npx playwright install chromium
npm run test:e2e
npm run format:check
```

The core can be tested without desktop libraries. Workspace Clippy requires the platform's Tauri prerequisites. For workspace tests, build the UI assets first with `npm run build`, then run `cargo test --workspace --locked`.

On Windows, append `--target x86_64-pc-windows-msvc` to Cargo test and Clippy commands, before Clippy's `--` separator. See [build verification](docs/ci/README.md) for the integrated CI checks and platform packaging.

The bundled preview fixture is generated by the same Rust engine used in the desktop app:

```sh
cargo run -p dfir-log-core --example export_demo --locked > src/demo-data.json
```

Additional documentation: [architecture](docs/architecture.md), [UI/UX audit](docs/ui-ux-audit.md), [redesign delivery report](docs/ui-ux-redesign.md), [performance](docs/performance.md), and [Delivery 1 architecture][mvp1-architecture].

## License and acknowledgments

Project code is licensed under the [MIT License](LICENSE). The locally bundled Inter font is distributed under the [SIL Open Font License](public/licenses/Inter-OFL.txt). Dependency licenses remain applicable to their respective components.

[mvp1-acceptance]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/mvp-1-acceptance.md
[mvp1-formats]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/mvp-1-formats.md
[mvp1-validation]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/mvp-1-validation.md
[mvp1-privacy]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/privacy.md
[mvp1-roadmap]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/roadmap.md
[mvp1-architecture]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/architecture.md
[mvp1-workspace]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/minimal-workspace.md
[mvp1-builds]: https://github.com/a-cisse/dfir-log-explorer/blob/9684e828fbc9c84ff87a95d7504cb6f81aa934bc/docs/ci/README.md
