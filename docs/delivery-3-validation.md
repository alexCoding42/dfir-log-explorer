# Delivery 3 validation record

Recorded on October 19, 2026 for version 0.4.0, branch `codex/9-11-delivery-3`, based on accepted `main` at `33a3bdb`. All evidence used for testing is synthetic. Issues #9–#11 hold the live PR, CI evidence and session handover; this report records local results rather than claiming human acceptance.

## Executed local checks

| Check                                                                      | Actual result                                                                                                                                                                           |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test --workspace --locked --offline`                                | Passed: 47 tests, including nine new investigation/persistence tests. Desktop and doc-test targets compile.                                                                             |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed.                                                                                                                                                                                 |
| `cargo fmt --all --check`                                                  | Passed.                                                                                                                                                                                 |
| `npm run build`                                                            | Passed: TypeScript and production Vite assets.                                                                                                                                          |
| `npm test`                                                                 | Passed: five preview tests.                                                                                                                                                             |
| `npm run format:check` and `git diff --check`                              | Passed, including the final acceptance/validation documents.                                                                                                                            |
| Chromium UI workflows                                                      | Passed: 17 workflows, including four Delivery 3 scenarios.                                                                                                                              |
| `npm run desktop:build -- --debug --bundles deb`                           | Passed: Linux v0.4.0 `.deb`, approximately 26.9 MiB, without installation or service changes.                                                                                           |
| Native Linux baseline                                                      | Passed: real bridge/import/query/detail, parsing states, statistics, sorting and warning filter.                                                                                        |
| Native Linux persistent-case regression                                    | Passed: real save/open dialogs, two additional file imports, source metadata, reopening, UTC/JSON filters and inspector provenance.                                                     |
| Native Linux security-reader regression                                    | Passed after synchronizing the harness with enabled UI controls: four real file imports, multiline ranges, reader/attribute filters and save/new/open.                                  |
| Native Linux saved-search workflow                                         | Passed: create/apply/update/delete through React and Tauri, real save/new/open dialogs, persisted filters, native aggregates, explicit missing-source rejection and unchanged evidence. |

The new Rust tests verify complete-query breakdowns beyond pagination, deterministic ties, missing fields and timestamps, combined exact filters, half-open UTC boundaries, bounded grouping values, unrecognized reader categories, unchanged evidence during search edits, temporary-case promotion/reopening, unchanged old-case files on open/list, invalid names/queries, unavailable sources, unsupported/malformed/unknown definitions and serialized-size rejection.

The browser suite covers the existing compact workspace, themes, filters, source/reader controls, inspector and timeline behavior plus Delivery 3 zoom/back/reset, chained breakdown filters, named-query management and Escape/focus restoration. Browser searches are memory-only and do not establish native persistence by themselves.

## Test environment and corrected failures

Rust 1.99.0, Node.js 24 and locked project dependencies were used. The prior temporary Rust/browser tools were absent, so isolated toolchains and Chromium were prepared under `/tmp`; shell profiles, global Git identity, credentials and production service configuration were unchanged. The Rust dependency download succeeded before subsequent offline checks. An initial browser-server attempt was blocked by the sandbox's local listener restrictions; the authorized local test ran with the required tool approval.

The first native baseline exposed a real coordination issue: loading saved-search definitions set the import/exclusive busy flag and could reject the initial event query. Read operations now share the serialized engine mutex without reserving that flag; search writes remain exclusive. The native baseline and persistence regressions passed after the fix.

The security-reader harness initially attempted the next file selection before the previous import had re-enabled its controls. It now waits for enabled controls before dispatching select/click actions. The complete reader scenario then passed. The first saved-search workflow reached successful create/save/reopen/apply/update/delete and evidence checks, but its expected missing-source error was interpreted by the shared WebDriver helper as an exception. The harness now asserts that expected exception explicitly. No failed attempt is counted as a successful complete run.

The native workflows use an isolated Xvfb display, DBus session, tauri-driver, software compositing and temporary case files. Native dialog keystrokes are test-only; no filesystem path API or permission is added to the webview. Test tooling is stopped after each test session. No package is installed or deployed.

## Limits and remaining acceptance

- Local Rust/native builds use the debug profile. Release-mode Linux and Windows checks/installers run through the PR's existing CI workflow; its historical workflow ID is preserved. A started or queued CI job is not a successful result.
- Interactive Windows application acceptance, a fresh native offline-traffic audit and representative large-volume benchmarks were not executed in this session.
- Aggregates still scan uncached query scopes; top lists contain at most ten values per dimension, and missing values are counted separately. See the [acceptance guide](delivery-3-acceptance.md) for exact filtering, payload and resource boundaries.
- Evidence schema 1 remains readable. The optional saved-search extension uses independent definition version 1; unknown definitions are errors, not automatic migrations. Searches and cases remain unencrypted.
- Human Delivery 3 acceptance and explicit merge authorization remain required. No merge or Delivery 4 implementation is authorized by these checks.
