import { useState, type ReactNode } from "react";
import {
  AlertTriangle,
  CalendarRange,
  RotateCcw,
  X,
  SlidersHorizontal,
} from "lucide-react";
import { statusLabels } from "./ParsingStatusBadge";
import type {
  FieldFilter,
  ParseStatus,
  SearchField,
  SourceSummary,
} from "./types";
export const searchFields: { key: SearchField; label: string }[] = [
  { key: "all", label: "All fields + raw" },
  { key: "raw", label: "Raw log" },
  { key: "parser", label: "Identified reader" },
  { key: "timestamp", label: "Timestamp" },
  { key: "sourceIp", label: "Client IP" },
  { key: "method", label: "HTTP method" },
  { key: "path", label: "Path / URL" },
  { key: "status", label: "HTTP status" },
  { key: "responseBytes", label: "Response bytes" },
  { key: "protocol", label: "Protocol" },
  { key: "referrer", label: "Referer" },
  { key: "userAgent", label: "User-Agent" },
];
export function LogFilterBar({
  source,
  sourceSelector,
  savedSearches,
  advanced,
  extraChips,
  advancedActive,
  status,
  onStatus,
  warnings,
  onWarnings,
  warningCount,
  scope,
  onScope,
  fieldFilter,
  onFieldFilter,
  timeRange,
  onTimeRange,
  onReset,
  filtered,
  disabled,
  onError,
}: {
  source: SourceSummary;
  sourceSelector: ReactNode;
  savedSearches: ReactNode;
  advanced: ReactNode;
  extraChips: ReactNode;
  advancedActive: boolean;
  status: ParseStatus | null;
  onStatus: (v: ParseStatus | null) => void;
  warnings: boolean;
  onWarnings: () => void;
  warningCount: number;
  scope: SearchField;
  onScope: (v: SearchField) => void;
  fieldFilter: FieldFilter | null;
  onFieldFilter: (v: FieldFilter | null) => void;
  timeRange: [number | null, number | null];
  onTimeRange: (v: [number | null, number | null]) => void;
  onReset: () => void;
  filtered: boolean;
  disabled: boolean;
  onError: (v: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [timeOpen, setTimeOpen] = useState(false);
  const [fieldOpen, setFieldOpen] = useState(false);
  const [field, setField] = useState<SearchField>("sourceIp");
  const [value, setValue] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const iso = (v: number) =>
    new Date(v).toISOString().replace("T", " ").replace("Z", " UTC");
  return (
    <div className="filter-area">
      <div className="filter-bar">
        {sourceSelector}
        <div className="status-filters" aria-label="Parsing status filters">
          {([null, "parsed", "partial", "unparsed"] as const).map((v) => (
            <button
              key={v ?? "all"}
              aria-label={v ? statusLabels[v] : "All"}
              title="Parsing counts for the selected sources"
              className={v === status ? "active" : ""}
              aria-pressed={v === status}
              disabled={disabled}
              onClick={() => onStatus(v)}
            >
              {v ? statusLabels[v] : "All"}
              <span aria-hidden="true">
                {v === "parsed"
                  ? source.parsedEvents
                  : v === "partial"
                    ? source.partialEvents
                    : v === "unparsed"
                      ? source.unparsedEvents
                      : source.totalEvents}
              </span>
            </button>
          ))}
        </div>
        <span className="filter-divider" />
        <button
          className={`filter-button ${warnings ? "active" : ""}`}
          aria-pressed={warnings}
          disabled={disabled}
          onClick={onWarnings}
        >
          <AlertTriangle size={13} />
          Warnings <span>{warningCount}</span>
        </button>
        <button
          className={`text-button more-filters ${advancedActive ? "active" : ""}`}
          aria-label="Additional filters"
          aria-expanded={expanded}
          aria-controls="additional-filters"
          disabled={disabled}
          onClick={() => setExpanded(!expanded)}
        >
          <SlidersHorizontal size={14} />
          Filters{advancedActive && <span className="live-dot" />}
        </button>
        {savedSearches}
        {filtered && (
          <button
            className="text-button reset-filters"
            disabled={disabled}
            onClick={onReset}
          >
            <RotateCcw size={13} />
            Reset
          </button>
        )}
      </div>
      {expanded && (
        <div className="additional-filters" id="additional-filters">
          {advanced}
          <div className="advanced-controls">
            <label className="search-scope">
              <span>Search in</span>
              <select
                aria-label="Search field"
                disabled={disabled}
                value={scope}
                onChange={(e) => onScope(e.target.value as SearchField)}
              >
                {searchFields.map((f) => (
                  <option key={f.key} value={f.key}>
                    {f.label}
                  </option>
                ))}
              </select>
            </label>
            <button
              className="text-button"
              disabled={disabled}
              aria-expanded={fieldOpen}
              onClick={() => setFieldOpen(!fieldOpen)}
            >
              Field filter
            </button>
            <button
              className="text-button"
              disabled={disabled}
              aria-expanded={timeOpen}
              onClick={() => setTimeOpen(!timeOpen)}
            >
              <CalendarRange size={14} />
              Time range
            </button>
          </div>
        </div>
      )}
      {expanded && fieldOpen && (
        <form
          className="filter-form"
          onSubmit={(e) => {
            e.preventDefault();
            onFieldFilter({ field, value });
            setFieldOpen(false);
          }}
        >
          <label>
            Exact field
            <select
              aria-label="Exact field"
              value={field}
              onChange={(e) => setField(e.target.value as SearchField)}
            >
              {searchFields
                .filter((f) => f.key !== "all" && f.key !== "raw")
                .map((f) => (
                  <option key={f.key} value={f.key}>
                    {f.label}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Value
            <input
              aria-label="Exact value"
              value={value}
              maxLength={4096}
              onChange={(e) => setValue(e.target.value)}
            />
          </label>
          <button className="secondary-button" disabled={disabled}>
            Apply field filter
          </button>
        </form>
      )}
      {expanded && timeOpen && (
        <form
          className="filter-form"
          onSubmit={(e) => {
            e.preventDefault();
            const start = from ? Date.parse(`${from}Z`) : null;
            const end = to ? Date.parse(`${to}Z`) : null;
            if (
              (start !== null && !Number.isFinite(start)) ||
              (end !== null && !Number.isFinite(end)) ||
              (start !== null && end !== null && start >= end)
            ) {
              onError("Choose a valid UTC range with its end after its start.");
              return;
            }
            onTimeRange([start, end]);
            setTimeOpen(false);
          }}
        >
          <label>
            From (UTC, inclusive)
            <input
              aria-label="Time from UTC"
              type="datetime-local"
              step="0.001"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
            />
          </label>
          <label>
            To (UTC, exclusive)
            <input
              aria-label="Time to UTC"
              type="datetime-local"
              step="0.001"
              value={to}
              onChange={(e) => setTo(e.target.value)}
            />
          </label>
          <button className="secondary-button" disabled={disabled}>
            Apply time range
          </button>
        </form>
      )}
      {extraChips}
      {scope !== "all" && (
        <div className="filter-chips">
          <button
            className="filter-chip"
            disabled={disabled}
            onClick={() => onScope("all")}
          >
            Search: {searchFields.find((f) => f.key === scope)?.label}
            <X size={12} />
          </button>
        </div>
      )}
      {(fieldFilter || timeRange.some((v) => v !== null)) && (
        <div className="filter-chips">
          {fieldFilter && (
            <button
              className="filter-chip"
              disabled={disabled}
              title={`${fieldFilter.field} = ${fieldFilter.value}`}
              onClick={() => onFieldFilter(null)}
            >
              {searchFields.find((f) => f.key === fieldFilter.field)?.label} ={" "}
              <span>{fieldFilter.value}</span>
              <X size={12} />
            </button>
          )}
          {timeRange.some((v) => v !== null) && (
            <button
              className="filter-chip"
              disabled={disabled}
              onClick={() => onTimeRange([null, null])}
            >
              {timeRange[0] === null ? "Any start" : iso(timeRange[0])} →{" "}
              {timeRange[1] === null ? "Any end" : iso(timeRange[1])}
              <X size={12} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
