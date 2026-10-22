import { useState } from "react";
import { ChevronDown, ChevronUp, ChartNoAxesColumn } from "lucide-react";
import { formatNumber } from "./format";
import type { Statistics } from "./types";
function utc(value: number) {
  return new Date(value)
    .toISOString()
    .replace("T", " ")
    .replace(/Z$/, " UTC")
    .replace(".000 UTC", " UTC");
}
export function TimelineChart({
  stats,
  onRange,
  disabled,
  available,
  pending,
  filtered,
  timeRange,
  onZoom,
  onBack,
  onReset,
  canBack,
}: {
  stats: Statistics;
  onRange: (from: number, to: number) => void;
  disabled: boolean;
  available: boolean;
  pending: boolean;
  filtered: boolean;
  timeRange: [number | null, number | null];
  onZoom: (from: number, to: number) => void;
  onBack: () => void;
  onReset: () => void;
  canBack: boolean;
}) {
  const [action, setAction] = useState<"logs" | "zoom">("logs");
  const [expanded, setExpanded] = useState(true);
  const [view, setView] = useState<"parsing" | "http">("parsing");
  const [hovered, setHovered] = useState<number | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const buckets = stats.timeline;
  const labels =
    view === "parsing"
      ? ["Parsed", "Partial", "Unparsed"]
      : ["2xx", "3xx", "4xx", "5xx"];
  const colorClass = (index: number) =>
    view === "parsing" ? labels[index].toLowerCase() : `group-${index}`;
  const values = buckets.map((b) =>
    view === "parsing" ? [b.parsed, b.partial, b.unparsed] : b.responses,
  );
  const totals = labels.map((_, index) =>
    values.reduce((sum, bucket) => sum + bucket[index], 0),
  );
  const maximum = Math.max(
    1,
    ...values.map((v) => v.reduce((a, b) => a + b, 0)),
  );
  // Use whole event counts and round the scale to readable 1 / 2 / 5 steps.
  const rawStep = maximum / 4;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const step = Math.max(
    1,
    [1, 2, 5, 10].find((n) => n * magnitude >= rawStep)! * magnitude,
  );
  const scaleMaximum = Math.ceil(maximum / step) * step;
  const ticks = Array.from(
    { length: scaleMaximum / step + 1 },
    (_, i) => i * step,
  );
  const activeIndex = buckets.findIndex(
    (bucket) => bucket.startMs === (hovered ?? focused),
  );
  const activeBucket = buckets[activeIndex];
  const start = buckets[0]?.startMs ?? 0;
  const end = buckets.at(-1)?.endMs ?? start;
  return (
    <section
      className={`timeline-panel ${expanded ? "" : "collapsed"}`}
      aria-label="Event timeline"
      aria-busy={pending}
    >
      <div className="timeline-heading">
        <div>
          <button
            className="text-button"
            aria-expanded={expanded}
            onClick={() => setExpanded(!expanded)}
          >
            <ChartNoAxesColumn size={20} />
            <strong>Log volume over time</strong>
            {expanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>
          <p>
            {filtered
              ? "Matching events · current filters"
              : "All source events"}{" "}
            · UTC
          </p>
        </div>
        {expanded && (
          <div className="timeline-controls">
            <label htmlFor="timeline-distribution">Group by</label>
            <select
              id="timeline-distribution"
              aria-label="Timeline distribution"
              value={view}
              onChange={(e) => setView(e.target.value as typeof view)}
            >
              <option value="parsing">Parsing status</option>
              <option value="http">HTTP responses</option>
            </select>
          </div>
        )}
      </div>
      {expanded && (
        <div className="timeline-navigation">
          <label>
            Interval action{" "}
            <select
              aria-label="Timeline interval action"
              value={action}
              onChange={(e) => setAction(e.target.value as "logs" | "zoom")}
              disabled={disabled}
            >
              <option value="logs">View logs</option>
              <option value="zoom">Zoom in</option>
            </select>
          </label>
          <button
            className="secondary-button"
            disabled={disabled || !canBack}
            onClick={onBack}
          >
            Previous time range
          </button>
          <button
            className="text-button"
            disabled={disabled || timeRange.every((v) => v === null)}
            onClick={onReset}
          >
            Reset time range
          </button>
          <p>
            {timeRange.some((v) => v !== null)
              ? `${timeRange[0] === null ? "Unbounded start" : utc(timeRange[0])} → ${timeRange[1] === null ? "Unbounded end" : utc(timeRange[1])} (end excluded)`
              : "All timestamps · select Zoom in to refine an interval"}
          </p>
        </div>
      )}
      {expanded &&
        (!available ? (
          <div className="timeline-empty" role="status">
            {pending
              ? "Updating event distribution…"
              : "Timeline unavailable. Retry the query."}
          </div>
        ) : buckets.length ? (
          <>
            <div className="timeline-summary">
              <span>
                <strong>
                  {formatNumber(stats.total - stats.missingTimestamp)}
                </strong>{" "}
                timestamped events
              </span>
              <span>
                {formatNumber(stats.missingTimestamp)} without valid timestamp
              </span>
              <span className="timeline-unit">
                {view === "parsing" ? "Events" : "HTTP responses"} per interval
              </span>
            </div>
            <div
              className="timeline-plot"
              onMouseLeave={() => setHovered(null)}
            >
              <div className="timeline-y-axis" aria-label="Volume scale">
                {ticks.map((tick) => (
                  <span
                    key={tick}
                    style={{ bottom: `${(tick / scaleMaximum) * 100}%` }}
                  >
                    {formatNumber(tick)}
                  </span>
                ))}
              </div>
              <div className="timeline-grid" aria-hidden="true">
                {ticks.map((tick) => (
                  <i
                    key={tick}
                    style={{ bottom: `${(tick / scaleMaximum) * 100}%` }}
                  />
                ))}
              </div>
              <div
                className="histogram"
                aria-label="Click an interval to filter events"
              >
                {buckets.map((bucket, i) => {
                  const count = values[i].reduce((a, b) => a + b, 0);
                  return (
                    <button
                      key={bucket.startMs}
                      disabled={disabled}
                      className={`histogram-bin ${activeIndex === i ? "active" : ""}`}
                      aria-label={`Filter interval ${utc(bucket.startMs)} to ${utc(bucket.endMs)} · ${formatNumber(count)} ${view === "parsing" ? "events" : "HTTP responses"}`}
                      title={`${utc(bucket.startMs)} → ${utc(bucket.endMs)}\n${bucket.parsed + bucket.partial + bucket.unparsed} events · Parsed ${bucket.parsed}, Partial ${bucket.partial}, Unparsed ${bucket.unparsed}\nHTTP ${bucket.responses.map((n, j) => `${j + 2}xx: ${n}`).join(", ")}`}
                      onMouseEnter={() => setHovered(bucket.startMs)}
                      onFocus={() => setFocused(bucket.startMs)}
                      onBlur={() => setFocused(null)}
                      onClick={() =>
                        (action === "zoom" ? onZoom : onRange)(
                          Math.max(bucket.startMs, timeRange[0] ?? -Infinity),
                          Math.min(bucket.endMs, timeRange[1] ?? Infinity),
                        )
                      }
                    >
                      {values[i].map((value, j) => (
                        <span
                          key={j}
                          className={colorClass(j)}
                          style={{ height: `${(value / scaleMaximum) * 100}%` }}
                        />
                      ))}
                    </button>
                  );
                })}
              </div>
            </div>
            <div className="timeline-axis" aria-label="Time scale in UTC">
              {[0, 0.25, 0.5, 0.75, 1].map((position) => {
                const date = new Date(
                  start + (end - start) * position,
                ).toISOString();
                return (
                  <span key={position} style={{ left: `${position * 100}%` }}>
                    <strong>
                      {date.slice(11, end - start < 5000 ? 23 : 19)}
                    </strong>
                    <span>{date.slice(0, 10)}</span>
                  </span>
                );
              })}
            </div>
            <div className="timeline-footer">
              <div className="chart-legend" aria-label="Distribution legend">
                {labels.map((label, i) => (
                  <span key={label}>
                    <i className={colorClass(i)} />
                    {label} <strong>{formatNumber(totals[i])}</strong>
                  </span>
                ))}
              </div>
              <p>
                {action === "zoom"
                  ? "Click an interval to zoom in. Other filters stay active."
                  : "Click an interval to view its logs."}
              </p>
            </div>
            {view === "http" && (
              <p className="timeline-scope-note">
                Only 2xx–5xx responses with valid timestamps appear in this
                chart.
              </p>
            )}
            <div className="timeline-readout" aria-label="Interval details">
              {activeBucket ? (
                <>
                  <strong>
                    {utc(activeBucket.startMs)} → {utc(activeBucket.endMs)}
                  </strong>
                  <span>
                    {formatNumber(
                      values[activeIndex].reduce((a, b) => a + b, 0),
                    )}{" "}
                    {view === "parsing" ? "events" : "HTTP responses"} ·{" "}
                    {labels
                      .map(
                        (label, i) =>
                          `${label}: ${formatNumber(values[activeIndex][i])}`,
                      )
                      .join(" · ")}
                  </span>
                </>
              ) : (
                <span>
                  Hover or focus an interval to inspect its volume and
                  distribution.
                </span>
              )}
            </div>
          </>
        ) : (
          <div className="timeline-empty" role="status">
            <strong>
              {formatNumber(stats.missingTimestamp)} without valid timestamp
            </strong>
            {stats.total === 0
              ? "No events match the current filters. Adjust them in Explore to view a timeline."
              : "No valid timestamps in these results. Events remain available in the table."}
          </div>
        ))}
    </section>
  );
}
