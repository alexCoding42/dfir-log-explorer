import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Copy,
  Filter,
  X,
  FileText,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { displayTimestamp } from "./format";
import { ParsingStatusBadge } from "./ParsingStatusBadge";
import type {
  EventDetail,
  FieldFilter,
  JsonFilter,
  ParseStatus,
  SearchField,
  SourceSummary,
} from "./types";
type Tab = "Details" | "Raw Log" | "Parse Diagnostics";
export function DetailPanel({
  detail,
  source,
  onClose,
  onFilter,
  onJsonFilter,
  onParsingFilter,
  onNavigate,
  hasPrevious,
  hasNext,
  disabled,
}: {
  detail: EventDetail;
  source: SourceSummary;
  onClose: () => void;
  onFilter: (filter: FieldFilter) => void;
  onJsonFilter: (filter: JsonFilter) => void;
  onParsingFilter: (status: ParseStatus) => void;
  onNavigate: (direction: number) => void;
  hasPrevious: boolean;
  hasNext: boolean;
  disabled: boolean;
}) {
  const [tab, setTab] = useState<Tab>("Details");
  const [message, setMessage] = useState("");
  useEffect(() => {
    setMessage("");
  }, [detail.id]);
  const fields: {
    key: SearchField;
    label: string;
    value: string | number | null;
  }[] = [
    {
      key: "timestamp",
      label: "Timestamp / source offset",
      value: detail.timestamp,
    },
    { key: "sourceIp", label: "Client IP / host", value: detail.sourceIp },
    { key: "method", label: "HTTP method", value: detail.method },
    { key: "path", label: "Path / URL", value: detail.path },
    { key: "status", label: "HTTP status", value: detail.status },
    {
      key: "responseBytes",
      label: "Response bytes",
      value: detail.responseBytes,
    },
    { key: "protocol", label: "Protocol", value: detail.protocol },
    { key: "referrer", label: "Referer", value: detail.referrer },
    { key: "userAgent", label: "User-Agent", value: detail.userAgent },
  ];
  const available = fields.filter((f) => f.value !== null);
  async function copy(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      setMessage("Copied to clipboard.");
    } catch {
      setMessage(
        "Clipboard unavailable. Select the text and copy it manually.",
      );
    }
  }
  const raw = (
    <div className="raw-viewer">
      <div className="raw-heading">
        <strong>
          <FileText size={14} />
          Original log event
        </strong>
        <button className="text-button" onClick={() => void copy(detail.raw)}>
          <Copy size={13} />
          Copy raw log
        </button>
      </div>
      <pre className="raw-event" tabIndex={0}>
        {detail.raw}
      </pre>
      {detail.raw === "" && <p className="muted">Empty source line.</p>}
      {detail.rawHex && (
        <>
          <h3>Original bytes / hexadecimal</h3>
          <pre className="raw-event hex" tabIndex={0}>
            {detail.rawHex}
          </pre>
        </>
      )}
    </div>
  );
  return (
    <aside
      className="detail-panel"
      aria-label="Event details"
      aria-busy={disabled}
    >
      <div className="detail-heading">
        <div>
          <span className="eyebrow">EVENT INSPECTOR</span>
          <h2>
            {detail.endLineNumber && detail.endLineNumber > detail.lineNumber
              ? `Lines ${detail.lineNumber}–${detail.endLineNumber}`
              : `Line ${detail.lineNumber}`}
          </h2>
        </div>
        <div className="detail-navigation">
          <button
            className="icon-button"
            aria-label="Previous event"
            disabled={!hasPrevious || disabled}
            onClick={() => onNavigate(-1)}
          >
            <ChevronLeft size={16} />
          </button>
          <button
            className="icon-button"
            aria-label="Next event"
            disabled={!hasNext || disabled}
            onClick={() => onNavigate(1)}
          >
            <ChevronRight size={16} />
          </button>
          <button
            className="icon-button"
            aria-label="Close event details"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
      </div>
      <div className="detail-meta">
        <ParsingStatusBadge status={detail.parseStatus} />
        <span className="parser-tag">{detail.parser.replaceAll("_", " ")}</span>
        <span className="mono">Offset {detail.byteOffset} B</span>
      </div>
      <div
        className="inspector-tabs"
        role="tablist"
        aria-label="Inspector views"
      >
        {(["Details", "Raw Log", "Parse Diagnostics"] as Tab[]).map((label) => (
          <button
            key={label}
            role="tab"
            id={`tab-${label.replaceAll(" ", "-")}`}
            aria-controls="inspector-content"
            aria-selected={tab === label}
            tabIndex={tab === label ? 0 : -1}
            onClick={() => setTab(label)}
            onKeyDown={(e) => {
              if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
              e.preventDefault();
              const tabs: Tab[] = ["Details", "Raw Log", "Parse Diagnostics"];
              const next =
                tabs[
                  (tabs.indexOf(tab) + (e.key === "ArrowRight" ? 1 : 2)) % 3
                ];
              setTab(next);
              document
                .getElementById(`tab-${next.replaceAll(" ", "-")}`)
                ?.focus();
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        className="inspector-content"
        id="inspector-content"
        role="tabpanel"
        aria-labelledby={`tab-${tab.replaceAll(" ", "-")}`}
      >
        {detail.parseError && tab !== "Raw Log" && (
          <div className="detail-warning">
            <AlertTriangle size={16} />
            <div>
              <strong>Parsing warning</strong>
              <p>{detail.parseError}</p>
            </div>
          </div>
        )}
        {tab === "Details" && (
          <>
            <h3>
              Extracted fields <span>{available.length}</span>
            </h3>
            <dl className="field-list">
              {fields.map((field) => (
                <div key={field.key}>
                  <dt>{field.label}</dt>
                  <dd>
                    <span>
                      {field.key === "timestamp"
                        ? displayTimestamp(field.value as string | null)
                        : (field.value ?? "—")}
                    </span>
                    {field.value !== null && (
                      <div className="field-actions">
                        <button
                          className="icon-button"
                          aria-label={`Copy ${field.label}`}
                          onClick={() => void copy(String(field.value))}
                        >
                          <Copy size={12} />
                        </button>
                        <button
                          className="icon-button"
                          aria-label={`Filter by ${field.label}`}
                          disabled={
                            disabled || String(field.value).length > 4096
                          }
                          title="Match this complete field value"
                          onClick={() =>
                            onFilter({
                              field: field.key,
                              value: String(field.value),
                            })
                          }
                        >
                          <Filter size={12} />
                        </button>
                      </div>
                    )}
                  </dd>
                </div>
              ))}
            </dl>
            {detail.jsonFields && Object.keys(detail.jsonFields).length > 0 && (
              <>
                <h3>Reader attributes / JSON fields</h3>
                <dl className="field-list">
                  {Object.entries(detail.jsonFields).map(([key, value]) => {
                    const scalar =
                      typeof value === "string" ||
                      typeof value === "number" ||
                      typeof value === "boolean";
                    const text =
                      typeof value === "string" ? value : JSON.stringify(value);
                    return (
                      <div key={key}>
                        <dt>{key}</dt>
                        <dd>
                          <span>{text}</span>
                          {scalar && (
                            <div className="field-actions">
                              <button
                                className="icon-button"
                                aria-label={`Copy attribute ${key}`}
                                onClick={() => void copy(text)}
                              >
                                <Copy size={12} />
                              </button>
                              <button
                                className="icon-button"
                                aria-label={`Filter by attribute ${key}`}
                                disabled={
                                  disabled ||
                                  key.length > 4096 ||
                                  text.length > 4096
                                }
                                onClick={() =>
                                  onJsonFilter({ key, value: text })
                                }
                              >
                                <Filter size={12} />
                              </button>
                            </div>
                          )}
                        </dd>
                      </div>
                    );
                  })}
                </dl>
              </>
            )}
            {raw}
          </>
        )}
        {tab === "Raw Log" && raw}
        {tab === "Parse Diagnostics" && (
          <>
            <h3>Parser result</h3>
            <dl className="field-list">
              <div>
                <dt>Selected parser</dt>
                <dd>{source.parserMode.replaceAll("_", " ")}</dd>
              </div>
              <div>
                <dt>Parser identified</dt>
                <dd>{detail.parser}</dd>
              </div>
              <div>
                <dt>Extracted fields</dt>
                <dd>
                  {available.map((f) => f.label).join(", ") ||
                    "No structured fields extracted"}
                </dd>
              </div>
              <div>
                <dt>Diagnostics</dt>
                <dd>
                  {detail.parseError ??
                    "No validation warning reported by the engine."}
                </dd>
              </div>
            </dl>
            <p className="detail-note">
              Missing optional fields do not indicate a parsing failure. Parsing
              quality and HTTP response codes are independent.
            </p>
            <button
              className="secondary-button"
              disabled={disabled}
              onClick={() => onParsingFilter(detail.parseStatus)}
            >
              <Filter size={14} />
              Find same parsing status
            </button>
            {raw}
          </>
        )}
        <div className="event-provenance">
          <h3>Provenance</h3>
          <span>
            Source #{source.id} · {source.name}
          </span>
          <span>{source.provenance}</span>
          <small className="mono">SHA-256: {source.sha256}</small>
          <span>
            Source{" "}
            {detail.endLineNumber && detail.endLineNumber > detail.lineNumber
              ? `lines ${detail.lineNumber}–${detail.endLineNumber}`
              : `line ${detail.lineNumber}`}{" "}
            · byte offset {detail.byteOffset}
          </span>
          <small>
            Raw bytes are retained in the local session. Display text excludes
            the final line terminator; internal line breaks are retained.
            Invalid UTF-8 uses replacement characters.
          </small>
        </div>
      </div>
      {message && (
        <div className="copy-message" role="status">
          {message}
        </div>
      )}
    </aside>
  );
}
