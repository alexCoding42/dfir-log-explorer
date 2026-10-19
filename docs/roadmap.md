# Incremental deliveries

The active roadmap uses **delivery** instead of MVP. Delivery numbers 0–5 retain their historical meaning; they are not semantic application versions. Existing `mvp-*` file paths, branches, workflow IDs and published review records remain valid historical identifiers.

Each delivery is independently testable with a documented scope, acceptance criteria, checks and limitations. The user must explicitly accept a delivery **and separately authorize starting the next one**. Pull requests are never merged automatically. A roadmap migration, milestone, Issue, estimate or passing CI does not authorize feature development.

## Tracking model

- This document defines direction, delivery boundaries, historical references and human acceptance gates.
- [Repository Issues](https://github.com/a-cisse/dfir-log-explorer/issues) hold actionable requirements, proposed acceptance criteria, dependencies, decisions and session handovers.
- [Project 4 — DFIR Log Explorer - Development](https://github.com/users/a-cisse/projects/4) holds live Status, Priority and Estimate. Repository milestones group the remaining deliveries. Follow the live records rather than treating this migration snapshot as permission to work.
- Future work starts in **Backlog**. **Todo** requires analysis, readiness and explicit authorization; **In Progress** means development has actually started; **In Review** requires an open, ready PR. **Blocked** requires a documented obstacle. **Done** requires explicit human acceptance and integration, not just green tests or a merged PR.
- The backlog is continuous: no sprint calendar, promised dates or milestone deadlines have been invented. The exploratory milestone is a grouping of ideas, not a committed delivery.
- Priorities and 1/2/3/5/8-point estimates are initial planning judgments. Revisit them after analysis, especially for distant work. Test plans in a Backlog Issue are future verification, not claims that tests ran.

The migration itself is tracked in [#7](https://github.com/a-cisse/dfir-log-explorer/issues/7). Feature Issues do not authorize their own implementation. Important decisions, test/CI evidence and session handovers stay linked to the Issue and PR; preserve the history when the plan changes.

## Delivery boundaries

| Delivery   | Scope                                                                                                                                                                                                                                                                         | Acceptance focus                                                | Tracking                                                                                                                                                                                                                                                        |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Delivery 0 | Temporary single-source sessions, Apache/text readers, literal search, virtualized pages, raw detail, warnings, progress and cancellation. Its UI redesign also brought basic structured/time filters, global sorting, statistics and a bounded interactive timeline forward. | Correctness, usability, offline operation and measured limits   | Historical [PR #1](https://github.com/a-cisse/dfir-log-explorer/pull/1) and [PR #2](https://github.com/a-cisse/dfir-log-explorer/pull/2); accepted and integrated, no artificial historical tickets                                                             |
| Delivery 1 | Persistent cases, multiple sources, Nginx/JSON Lines readers, structured filters, timestamp rules and the compact default workspace                                                                                                                                           | Persistence, source management and query reliability            | Historical [PR #3](https://github.com/a-cisse/dfir-log-explorer/pull/3), [README #4](https://github.com/a-cisse/dfir-log-explorer/pull/4) and [Statistics #5](https://github.com/a-cisse/dfir-log-explorer/pull/5), integrated; no recreated historical tickets |
| Delivery 2 | Syslog RFC 5424/BSD, Netfilter/iptables, HAProxy HTTP and bounded native ModSecurity 2.x audit                                                                                                                                                                                | Verified extraction, provenance and format-specific diagnostics | [Delivery 2 — Security log readers](https://github.com/a-cisse/dfir-log-explorer/milestone/1); [#8](https://github.com/a-cisse/dfir-log-explorer/issues/8); existing [PR #6](https://github.com/a-cisse/dfir-log-explorer/pull/6) in review                     |
| Delivery 3 | Expanded timeline/statistics investigation workflows and saved searches; the basic timeline/statistics already exist                                                                                                                                                          | Useful investigation workflows beyond the current baseline      | [Delivery 3 — Exploration and saved searches](https://github.com/a-cisse/dfir-log-explorer/milestone/2); Backlog, not authorized                                                                                                                                |
| Delivery 4 | Custom local parsing profiles, preview, version history and import using a selected profile                                                                                                                                                                                   | Importing an additional agreed text format transparently        | [Delivery 4 — Local parsing profiles](https://github.com/a-cisse/dfir-log-explorer/milestone/3); Backlog, not authorized                                                                                                                                        |
| Delivery 5 | Representative measurements, measured large-volume optimization, resumable ingestion, compression and exports                                                                                                                                                                 | Stability, correctness, resource use and throughput             | [Delivery 5 — Volume, ingestion and exports](https://github.com/a-cisse/dfir-log-explorer/milestone/4); Backlog, not authorized                                                                                                                                 |
| To explore | Binary readers, correlations and annotations; other needs only after agreement                                                                                                                                                                                                | Explicitly agreed scenarios and scope                           | [To explore — Future work to agree](https://github.com/a-cisse/dfir-log-explorer/milestone/5); no delivery commitment                                                                                                                                           |

The sequence may be adjusted by explicit agreement. Performance work remains permitted within any authorized delivery when needed to meet its acceptance criteria; it is not all deferred to Delivery 5. The migration does not invent additional reader formats, correlation rules, export formats or deadlines.

## Delivery 2: existing implementation in review

Delivery 0 was explicitly accepted on 2026-10-13. Delivery 1 and the README/Statistics changes are integrated into `main`. On 2026-10-15 the user authorized proceeding to Delivery 2 after checking integration; `main` matched `bb51d38` and there was no pending PR awaiting integration. Preserve these historical facts rather than fabricating retrospective completed Issues.

Delivery 2 version 0.3.0 is proposed in [PR #6](https://github.com/a-cisse/dfir-log-explorer/pull/6), on the historical branch `codex/mvp-2`. Its selected readers preserve raw evidence and physical source ranges; the review includes bounded native ModSecurity transactions, reader attributes and combined scalar filters. See the [format contract](https://github.com/a-cisse/dfir-log-explorer/blob/codex/mvp-2/docs/mvp-2-formats.md), [acceptance guide](https://github.com/a-cisse/dfir-log-explorer/blob/codex/mvp-2/docs/mvp-2-acceptance.md) and [validation record](https://github.com/a-cisse/dfir-log-explorer/blob/codex/mvp-2/docs/mvp-2-validation.md) on that branch until it is integrated.

[#8](https://github.com/a-cisse/dfir-log-explorer/issues/8) follows the **remaining validation and integration**, not a second implementation of the readers. Its 3-point estimate covers that remaining work only. Human acceptance and integration are still required; interactive Windows acceptance and other documented limits must remain explicit. No instruction to begin Delivery 3 is implied by this roadmap or its tracking migration.

## Delivery 3: exploration and saved searches

Prerequisites: explicit acceptance of Delivery 2 and a separate instruction to start Delivery 3. First refine the additional timeline/statistics scenarios with the user; do not redo the baseline delivered earlier. Acceptance includes useful investigation paths, consistent filtered results and reopening saved searches.

| Issue                                                         | Proposed work                                        | Initial points |
| ------------------------------------------------------------- | ---------------------------------------------------- | -------------- |
| [#9](https://github.com/a-cisse/dfir-log-explorer/issues/9)   | Delivery 3 — Extend timeline investigation workflows | 5              |
| [#10](https://github.com/a-cisse/dfir-log-explorer/issues/10) | Delivery 3 — Extend investigation statistics         | 5              |
| [#11](https://github.com/a-cisse/dfir-log-explorer/issues/11) | Delivery 3 — Save and reuse searches within a case   | 5              |

## Delivery 4: local parsing profiles

Prerequisites: acceptance of Delivery 3 and explicit authorization to start Delivery 4. Definition comes before preview/versioning; import depends on all three. Agree the first target format and profile model before implementation. Acceptance must demonstrate an agreed additional text format while preserving raw content, diagnostics and profile-version provenance.

| Issue                                                         | Proposed work                                            | Initial points |
| ------------------------------------------------------------- | -------------------------------------------------------- | -------------- |
| [#12](https://github.com/a-cisse/dfir-log-explorer/issues/12) | Delivery 4 — Define and edit local parsing profiles      | 5              |
| [#13](https://github.com/a-cisse/dfir-log-explorer/issues/13) | Delivery 4 — Preview a profile on a log sample           | 3              |
| [#14](https://github.com/a-cisse/dfir-log-explorer/issues/14) | Delivery 4 — Persist and version parsing profiles        | 3              |
| [#15](https://github.com/a-cisse/dfir-log-explorer/issues/15) | Delivery 4 — Import a text format using a custom profile | 5              |

## Delivery 5: volume, ingestion and exports

Prerequisites: acceptance of Delivery 4 and explicit authorization to start Delivery 5. Measure before selecting optimizations. Resumable ingestion requires an explicit storage/transaction design; compression/export formats and resource limits need agreement. Do not promise performance targets before measurement.

| Issue                                                         | Proposed work                                             | Initial points |
| ------------------------------------------------------------- | --------------------------------------------------------- | -------------- |
| [#16](https://github.com/a-cisse/dfir-log-explorer/issues/16) | Delivery 5 — Establish representative volume measurements | 3              |
| [#17](https://github.com/a-cisse/dfir-log-explorer/issues/17) | Delivery 5 — Optimize measured large-volume bottlenecks   | 5              |
| [#18](https://github.com/a-cisse/dfir-log-explorer/issues/18) | Delivery 5 — Resume interrupted ingestion                 | 8              |
| [#19](https://github.com/a-cisse/dfir-log-explorer/issues/19) | Delivery 5 — Import supported compressed sources          | 5              |
| [#20](https://github.com/a-cisse/dfir-log-explorer/issues/20) | Delivery 5 — Export investigation results                 | 5              |

## To explore: uncommitted ideas

These are the existing Later themes, not a new delivery or authorization. The estimates are particularly uncertain; agree a scenario, first supported format where relevant, acceptance boundaries and an assigned delivery before implementation. Do not create a catch-all ticket for unspecified future requirements.

| Issue                                                         | Proposed work                                          | Initial points |
| ------------------------------------------------------------- | ------------------------------------------------------ | -------------- |
| [#21](https://github.com/a-cisse/dfir-log-explorer/issues/21) | To explore — Binary log readers                        | 8              |
| [#22](https://github.com/a-cisse/dfir-log-explorer/issues/22) | To explore — Correlations between investigation events | 8              |
| [#23](https://github.com/a-cisse/dfir-log-explorer/issues/23) | To explore — Investigation event and case annotations  | 5              |

## Historical terminology and acceptance records

The former MVP 0–5 stages map directly to Delivery 0–5; the former Later line maps to To explore. Historical guides such as [Delivery 0 acceptance](mvp-0-acceptance.md), [Delivery 1 acceptance](mvp-1-acceptance.md), [Delivery 1 validation](mvp-1-validation.md), the [minimal workspace report](minimal-workspace.md) and [Statistics report](statistics-design.md) remain at their existing paths. Their original wording and recorded results are retained; none is a new test or acceptance claim.
