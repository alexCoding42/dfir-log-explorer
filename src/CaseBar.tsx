import { useEffect, useRef, useState } from "react";
import { FolderOpen, Save, Plus, ChevronDown } from "lucide-react";
import type { CaseSummary, JsonFilter, ParserMode } from "./types";

export function CaseBar({
  info,
  disabled,
  onSave,
  onOpen,
  onNew,
}: {
  info: CaseSummary;
  disabled: boolean;
  onSave: () => void;
  onOpen: () => void;
  onNew: () => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const outside = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    };
    document.addEventListener("mousedown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  function action(handler: () => void) {
    setOpen(false);
    handler();
  }
  return (
    <section ref={root} className="case-bar" aria-label="Case management">
      <button
        ref={trigger}
        className="secondary-button case-toggle"
        aria-label="Case actions"
        aria-expanded={open}
        aria-controls="case-actions"
        disabled={disabled}
        onClick={() => setOpen(!open)}
      >
        <FolderOpen size={14} />
        Case
        <ChevronDown size={12} />
      </button>
      {open && (
        <div className="case-popover" id="case-actions">
          <div className="case-title">
            <strong>{info.name}</strong>
            <span title={info.path ?? undefined}>
              {info.path
                ? "Saved locally · imports are saved automatically"
                : "Temporary case · save to keep your analysis"}{" "}
              · {info.sources.length} sources
            </span>
          </div>
          <button
            className="text-button"
            data-case-action="save"
            disabled={disabled || info.path !== null}
            onClick={() => action(onSave)}
          >
            <Save size={14} />
            Save case as…
          </button>
          <button
            className="text-button"
            data-case-action="open"
            disabled={disabled}
            onClick={() => action(onOpen)}
          >
            <FolderOpen size={14} />
            Open case…
          </button>
          <button
            className="text-button"
            data-case-action="new"
            disabled={disabled}
            onClick={() => action(onNew)}
          >
            <Plus size={14} />
            New case
          </button>
        </div>
      )}
    </section>
  );
}

export function SourceFilters({
  parser,
  onParser,
  hasTimestamp,
  onTimestamp,
  jsonFilter,
  onJson,
  disabled,
}: {
  parser: ParserMode | null;
  onParser: (mode: ParserMode | null) => void;
  hasTimestamp: boolean | null;
  onTimestamp: (value: boolean | null) => void;
  jsonFilter: JsonFilter | null;
  onJson: (filter: JsonFilter | null) => void;
  disabled: boolean;
}) {
  return (
    <div className="source-filters">
      <label>
        Reader
        <select
          aria-label="Reader filter"
          disabled={disabled}
          value={parser ?? ""}
          onChange={(e) =>
            onParser((e.target.value || null) as ParserMode | null)
          }
        >
          <option value="">All readers</option>
          <option value="apache">Apache</option>
          <option value="nginx">Nginx</option>
          <option value="json_lines">JSON Lines</option>
          <option value="syslog">Syslog</option>
          <option value="iptables">Netfilter / iptables</option>
          <option value="haproxy">HAProxy HTTP</option>
          <option value="modsecurity">ModSecurity audit</option>
          <option value="text">Text</option>
        </select>
      </label>
      <label>
        Timestamp
        <select
          aria-label="Timestamp presence"
          disabled={disabled}
          value={hasTimestamp === null ? "" : String(hasTimestamp)}
          onChange={(e) =>
            onTimestamp(
              e.target.value === "" ? null : e.target.value === "true",
            )
          }
        >
          <option value="">Any timestamp</option>
          <option value="true">Valid timestamp</option>
          <option value="false">Missing / invalid timestamp</option>
        </select>
      </label>
      <form
        className="json-filter-form"
        onSubmit={(e) => {
          e.preventDefault();
          const data = new FormData(e.currentTarget);
          onJson({
            key: String(data.get("key")),
            value: String(data.get("value")),
          });
        }}
      >
        <label>
          Attribute / JSON field
          <input
            aria-label="Attribute or JSON field name"
            name="key"
            defaultValue={jsonFilter?.key ?? ""}
            placeholder="e.g. firewall.DPT or user"
            maxLength={4096}
            required
            disabled={disabled}
          />
        </label>
        <label>
          Equals
          <input
            aria-label="Attribute or JSON field value"
            name="value"
            defaultValue={jsonFilter?.value ?? ""}
            placeholder="Exact scalar value"
            maxLength={4096}
            disabled={disabled}
          />
        </label>
        <button className="secondary-button" disabled={disabled}>
          Apply attribute filter
        </button>
      </form>
    </div>
  );
}
