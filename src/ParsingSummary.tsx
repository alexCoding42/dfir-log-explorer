import { formatNumber } from "./format";
import { statusLabels } from "./ParsingStatusBadge";
import type { ParseStatus, SourceSummary, Statistics } from "./types";
export function ParsingSummary({
  source,
  stats,
  filtered,
  onFilter,
  active,
  disabled,
  available,
  pending,
}: {
  source: SourceSummary;
  stats: Statistics;
  filtered: boolean;
  onFilter: (status: ParseStatus | null) => void;
  active: ParseStatus | null;
  disabled: boolean;
  available: boolean;
  pending: boolean;
}) {
  const total = source.totalEvents;
  const counts = {
    parsed: source.parsedEvents,
    partial: source.partialEvents,
    unparsed: source.unparsedEvents,
  };
  return (
    <div className="summary-section">
      <div className="statistics-section-heading">
        <h3>Parsing quality</h3>
        <p>Source totals · select a status to filter the logs.</p>
      </div>
      <div className="parsing-summary" aria-label="Source parsing statistics">
        <button
          disabled={disabled}
          className="summary-total"
          onClick={() => onFilter(null)}
        >
          <span>Total events</span>
          <strong>{formatNumber(total)}</strong>
          <small>Source order preserved</small>
        </button>
        {(Object.keys(counts) as ParseStatus[]).map((status) => (
          <button
            disabled={disabled}
            key={status}
            className={`summary-stat ${status} ${active === status ? "selected" : ""}`}
            aria-pressed={active === status}
            onClick={() => onFilter(active === status ? null : status)}
          >
            <span>
              <i />
              {statusLabels[status]}
            </span>
            <strong>
              {formatNumber(counts[status])}
              <small>
                {total ? ((counts[status] / total) * 100).toFixed(1) : "0.0"}%
              </small>
            </strong>
            <div className="mini-track">
              <i
                style={{
                  width: `${total ? (counts[status] / total) * 100 : 0}%`,
                }}
              />
            </div>
          </button>
        ))}
        <div className="summary-warnings">
          <span>Warnings</span>
          <strong>{formatNumber(source.warningEvents)}</strong>
          <small>Counted independently</small>
        </div>
      </div>
      <div className="statistics-section-heading http-heading">
        <h3>HTTP activity</h3>
        <p>
          {filtered ? "Matching events · current filters" : "All source events"}
        </p>
      </div>
      {available ? (
        <div className="http-statistics" aria-label="Filtered HTTP statistics">
          <span className="stats-context">
            {filtered ? "MATCHING EVENTS" : "SOURCE OVERVIEW"}
          </span>
          <span>
            <strong>{formatNumber(stats.totalRequests)}</strong> requests
          </span>
          <span>
            <strong>{formatNumber(stats.uniqueIps)}</strong> unique clients
          </span>
          {stats.responses.map((count, index) => (
            <span key={index} className={`http-group group-${index}`}>
              <i />
              {index + 2}xx <strong>{formatNumber(count)}</strong>
            </span>
          ))}
          <span className="stats-note">
            HTTP response codes describe outcomes.
          </span>
        </div>
      ) : (
        <div className="http-statistics" role="status">
          {pending
            ? "Updating result statistics…"
            : "Result statistics unavailable. Retry the query."}
        </div>
      )}
    </div>
  );
}
