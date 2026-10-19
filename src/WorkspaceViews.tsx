import type { ReactNode } from "react";
import {
  Activity,
  ArrowRight,
  Braces,
  Check,
  CircleHelp,
  FileText,
  Monitor,
  ShieldCheck,
} from "lucide-react";
import { formatBytes, formatNumber } from "./format";
import { parsers } from "./workspace";
import type { ParserMode, SourceSummary } from "./types";
function EvidenceItem({
  source,
  onExplore,
}: {
  source: SourceSummary | null;
  onExplore: () => void;
}) {
  return (
    <section className="secondary-view">
      <h2>Active evidence</h2>
      {source ? (
        <>
          <div className="evidence-card">
            <FileText size={24} />
            <div>
              <h3>{source.name}</h3>
              <p>{source.provenance}</p>
            </div>
            <span className="ready-badge">
              <Check size={13} />
              Imported
            </span>
          </div>
          <dl className="evidence-fields">
            <div>
              <dt>Size</dt>
              <dd>
                {formatBytes(source.totalBytes)} (
                {formatNumber(source.totalBytes)} bytes)
              </dd>
            </div>
            <div>
              <dt>Events / source lines</dt>
              <dd>
                {formatNumber(source.totalEvents)} /{" "}
                {formatNumber(source.totalLines || source.totalEvents)}
              </dd>
            </div>
            <div>
              <dt>Parser selection</dt>
              <dd>{parsers.find((p) => p.mode === source.parserMode)?.name}</dd>
            </div>
            <div>
              <dt>Parsing result</dt>
              <dd>
                {source.parsedEvents} Parsed · {source.partialEvents} Partial ·{" "}
                {source.unparsedEvents} Unparsed · {source.warningEvents}{" "}
                warnings
              </dd>
            </div>
            <div>
              <dt>Import duration</dt>
              <dd>{(source.elapsedMs / 1000).toFixed(2)} seconds</dd>
            </div>
            <div>
              <dt>SHA-256</dt>
              <dd className="mono hash-value">{source.sha256}</dd>
            </div>
          </dl>
          <button className="primary-button" onClick={onExplore}>
            <Activity size={15} />
            Explore events
          </button>
        </>
      ) : (
        <p>No source imported. Choose a log file to begin.</p>
      )}
      <div className="view-note">
        <ShieldCheck size={18} />
        <div>
          <strong>Original content retained</strong>
          <p>
            Sources are opened read-only. Each event retains its original bytes,
            line number and byte offset. Import failures and cancellation
            preserve the previous dataset.
          </p>
          <p>
            Imports add sources to the active case. Save a temporary case to
            keep all imported evidence; saved cases reopen without the original
            files.
          </p>
        </div>
      </div>
    </section>
  );
}
export function EvidenceView({
  sources,
  onExplore,
}: {
  sources: SourceSummary[];
  onExplore: () => void;
}) {
  return (
    <div>
      {sources.length === 0 ? (
        <section className="secondary-view">
          <h2>Active evidence</h2>
          <p>No sources imported.</p>
        </section>
      ) : (
        sources.map((source) => (
          <EvidenceItem key={source.id} source={source} onExplore={onExplore} />
        ))
      )}
    </div>
  );
}
export function ParsersView({
  mode,
  disabled,
  onSelect,
}: {
  mode: ParserMode;
  disabled: boolean;
  onSelect: (mode: ParserMode) => void;
}) {
  return (
    <section className="secondary-view">
      <h2>Preconfigured parsers</h2>
      <p className="muted">
        Choose a reader for your next import. The active source retains its
        original parser selection.
      </p>
      <div className="parser-cards">
        {parsers.map((parser) => (
          <article
            className={
              mode === parser.mode ? "parser-card selected" : "parser-card"
            }
            key={parser.mode}
          >
            <Braces size={22} />
            <h3>{parser.name}</h3>
            <p>{parser.description}</p>
            <small>{parser.formats}</small>
            <button
              className="secondary-button"
              disabled={disabled}
              onClick={() => {
                onSelect(parser.mode);
              }}
            >
              {mode === parser.mode ? "Use selected parser" : "Select parser"}
              <ArrowRight size={14} />
            </button>
          </article>
        ))}
      </div>
      <div className="view-note">
        <CircleHelp size={18} />
        <p>
          Apache Error Logs, compressed and binary formats, custom parsers and
          arbitrary multiline grouping are not supported. Native ModSecurity 2.x
          audit transactions support explicit A–Z grouping.
        </p>
      </div>
    </section>
  );
}
export function SettingsView({
  themeControl,
  collapsed,
  onToggle,
  onHelp,
}: {
  themeControl: ReactNode;
  collapsed: boolean;
  onToggle: () => void;
  onHelp: () => void;
}) {
  return (
    <section className="secondary-view">
      <h2>Appearance</h2>
      <div className="setting-row">
        <div>
          <strong>Theme</strong>
          <p>Light by default. System follows your device preference.</p>
        </div>
        {themeControl}
      </div>
      <div className="setting-row">
        <div>
          <strong>Compact navigation</strong>
          <p>Give the events table more room.</p>
        </div>
        <button
          className="secondary-button"
          aria-pressed={collapsed}
          onClick={onToggle}
        >
          {collapsed ? "Expand sidebar" : "Collapse sidebar"}
        </button>
      </div>
      <div className="view-note">
        <Monitor size={18} />
        <p>
          Theme, sidebar width and visible columns are saved on this device. Log
          searches and source content are never saved in browser preferences.
        </p>
      </div>
      <h2>Session</h2>
      <p>
        Local cases with multiple sources · save and reopen your analysis · no
        telemetry or cloud backend.
      </p>
      <button className="secondary-button" onClick={onHelp}>
        <CircleHelp size={15} />
        Help & keyboard shortcuts
      </button>
    </section>
  );
}
