import { useEffect, useState } from "react";
import type {
  Bridge,
  CaseSummary,
  ProfileDefinition,
  ProfilePreviewRow,
  ProfileRevision,
} from "./types";

const initialDefinition: ProfileDefinition = {
  version: 1,
  name: "Application log",
  pattern: String.raw`(?P<timestamp>[^|]+?) \| (?P<level>[A-Z]+) \| (?P<message>.*)`,
};
const initialSample =
  "2026-11-11T12:00:00Z | INFO | Server ready\ninvalid-date | WARN | Clock needs review\nUnrecognized record\n";

export function ParsingProfiles({
  bridge,
  info,
  disabled,
  onSelect,
  onBusy,
}: {
  bridge: Bridge;
  info: CaseSummary;
  disabled: boolean;
  onSelect: (profile: ProfileRevision) => void;
  onBusy: (busy: boolean) => void;
}) {
  const [profiles, setProfiles] = useState<ProfileRevision[]>([]);
  const [selected, setSelected] = useState<ProfileRevision | null>(null);
  const [draft, setDraft] = useState(initialDefinition);
  const [sample, setSample] = useState(initialSample);
  const [rows, setRows] = useState<ProfilePreviewRow[] | null>(null);
  const [working, setWorking] = useState(false);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    let current = true;
    setReady(false);
    setProfiles([]);
    setSelected(null);
    setDraft(initialDefinition);
    setRows(null);
    setError("");
    bridge
      .parsingProfiles()
      .then((items) => {
        if (current) {
          setProfiles(items);
          setReady(true);
        }
      })
      .catch((reason) => {
        if (current) setError(String(reason));
      });
    return () => {
      current = false;
    };
  }, [bridge, info]);
  const locked = disabled || working || !ready || !bridge.isDesktop;
  const latest = selected
    ? Math.max(
        ...profiles.filter((p) => p.id === selected.id).map((p) => p.revision),
      )
    : 0;
  const changed =
    selected &&
    (draft.name !== selected.definition.name ||
      draft.pattern !== selected.definition.pattern);
  function choose(profile: ProfileRevision | null) {
    setSelected(profile);
    setDraft(profile?.definition ?? initialDefinition);
    setRows(null);
    setError("");
    setMessage("");
  }
  async function run(action: "preview" | "create" | "revise") {
    if (locked) return;
    setWorking(true);
    onBusy(true);
    setError("");
    setMessage("");
    setRows(null);
    try {
      if (action === "preview") {
        setRows(await bridge.previewProfile(draft, sample));
        setMessage("Preview complete. No evidence was imported.");
      } else {
        const result = await bridge.putParsingProfile(
          action === "revise" ? selected : null,
          draft,
        );
        setProfiles(await bridge.parsingProfiles());
        setSelected(result);
        setDraft(result.definition);
        setMessage(
          `Saved ${result.definition.name}, revision ${result.revision}.`,
        );
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      setWorking(false);
      onBusy(false);
    }
  }
  return (
    <section className="profile-workspace" aria-labelledby="profiles-heading">
      <h2 id="profiles-heading">Local parsing profiles</h2>
      <p>
        Describe a single-line text format, check a sample, then select a saved
        revision for import. Imported sources retain the exact definition used.
      </p>
      <p className="view-note">
        {!bridge.isDesktop
          ? "Custom profiles and preview require the desktop application. The browser preview uses bundled synthetic evidence only."
          : info.path
            ? "Profiles are stored in this case. Each save adds an immutable revision."
            : "Temporary case: save the case to retain your profiles and their history."}
      </p>
      <div className="profile-layout">
        <div className="profile-editor">
          <label>
            Profile
            <select
              aria-label="Parsing profile"
              disabled={locked}
              value={selected?.id ?? ""}
              onChange={(e) => {
                const id = Number(e.target.value);
                choose(
                  [...profiles].reverse().find((p) => p.id === id) ?? null,
                );
              }}
            >
              <option value="">New profile</option>
              {profiles
                .filter((p) => p.revision === 1)
                .map((p) => {
                  const last = [...profiles]
                    .reverse()
                    .find((v) => v.id === p.id)!;
                  return (
                    <option key={p.id} value={p.id}>
                      {last.definition.name}
                    </option>
                  );
                })}
            </select>
          </label>
          {selected && (
            <label>
              Version history
              <select
                aria-label="Profile revision"
                disabled={locked}
                value={selected.revision}
                onChange={(e) =>
                  choose(
                    profiles.find(
                      (p) =>
                        p.id === selected.id &&
                        p.revision === Number(e.target.value),
                    )!,
                  )
                }
              >
                {profiles
                  .filter((p) => p.id === selected.id)
                  .map((p) => (
                    <option key={p.revision} value={p.revision}>
                      Revision {p.revision} — {p.definition.name}
                    </option>
                  ))}
              </select>
            </label>
          )}
          <label>
            Profile name
            <input
              aria-label="Profile name"
              disabled={locked}
              maxLength={120}
              value={draft.name}
              onChange={(e) => {
                setDraft({ ...draft, name: e.target.value });
                setRows(null);
              }}
            />
          </label>
          <label>
            Full-line pattern
            <textarea
              aria-label="Profile pattern"
              disabled={locked}
              value={draft.pattern}
              onChange={(e) => {
                setDraft({ ...draft, pattern: e.target.value });
                setRows(null);
              }}
              rows={5}
              spellCheck={false}
            />
          </label>
          <p className="muted">
            Use Rust regex named captures: <code>{"(?P<level>[A-Z]+)"}</code>.
            All captures must be named (1–32); patterns have a 4096-byte limit.
            Captures become reader attributes. The whole line must match.
          </p>
          <details>
            <summary>Normalized fields and limits</summary>
            <p>
              <code>timestamp</code> requires RFC 3339 with a timezone.{" "}
              <code>source_ip</code>, <code>method</code>, <code>path</code>,{" "}
              <code>protocol</code>, <code>status</code>,{" "}
              <code>response_bytes</code>, <code>referrer</code> and{" "}
              <code>user_agent</code> populate event fields. Invalid timestamps
              or numbers produce Partial records. Missing optional captures are
              allowed. Other names remain searchable reader attributes.
            </p>
            <p>
              50 profiles, 20 revisions each, 200 revisions per case. No
              scripts, lookaround, backreferences or multiline grouping.
            </p>
          </details>
          <div className="profile-actions">
            <button
              className="secondary-button"
              disabled={locked || !draft.name.trim()}
              onClick={() => void run("create")}
            >
              Save as new profile
            </button>
            <button
              className="secondary-button"
              disabled={
                locked || !selected || selected.revision !== latest || !changed
              }
              onClick={() => void run("revise")}
            >
              Save new revision
            </button>
          </div>
          {selected && selected.revision !== latest && (
            <p>
              Historical revision: select the latest revision to edit this
              profile, or use a different name to save a new profile.
            </p>
          )}
          {selected && (
            <>
              <p className="muted">
                Definition SHA-256{" "}
                <code className="hash-value">{selected.sha256}</code>
              </p>
              <button
                className="primary-button"
                disabled={locked || !!changed}
                onClick={() => onSelect(selected)}
              >
                Use revision {selected.revision} for next import
              </button>
              {changed && (
                <p>
                  Save your changes or reload a revision before selecting it for
                  import.
                </p>
              )}
            </>
          )}
        </div>
        <div className="profile-preview">
          <label>
            Sample text
            <textarea
              aria-label="Profile sample"
              disabled={locked}
              value={sample}
              onChange={(e) => {
                setSample(e.target.value);
                setRows(null);
              }}
              rows={8}
              spellCheck={false}
            />
          </label>
          <p className="muted">
            Paste up to 64 KiB / 100 physical lines. Preview uses the same
            reader as file import.
          </p>
          <button
            className="secondary-button"
            disabled={locked}
            onClick={() => void run("preview")}
          >
            {working ? "Working…" : "Preview profile"}
          </button>
          {rows && (
            <div
              className="profile-results"
              aria-label="Profile preview results"
            >
              <p>
                {rows.length} records ·{" "}
                {rows.filter((r) => r.fields.parseStatus === "parsed").length}{" "}
                Parsed ·{" "}
                {rows.filter((r) => r.fields.parseStatus === "partial").length}{" "}
                Partial ·{" "}
                {rows.filter((r) => r.fields.parseStatus === "unparsed").length}{" "}
                Unparsed
              </p>
              {rows.map((row) => (
                <article key={row.lineNumber} className="profile-result">
                  <strong>
                    Line {row.lineNumber} · byte {row.byteOffset} ·{" "}
                    {row.fields.parseStatus}
                  </strong>
                  <pre>{row.raw}</pre>
                  {row.fields.parseError && (
                    <p className="profile-diagnostic">
                      {row.fields.parseError}
                    </p>
                  )}
                  {row.fields.timestamp && (
                    <p>Timestamp: {row.fields.timestamp}</p>
                  )}
                  <pre>
                    {JSON.stringify(row.fields.jsonFields ?? {}, null, 2)}
                  </pre>
                </article>
              ))}
            </div>
          )}
        </div>
      </div>
      {error && (
        <p role="alert" className="profile-diagnostic">
          {error}
        </p>
      )}
      <p role="status">{message}</p>
    </section>
  );
}
