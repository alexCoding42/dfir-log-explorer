import type { GroupCount, Statistics, SourceSummary } from "./types";
import { formatNumber } from "./format";

export type Dimension = "sourceIp" | "path" | "sourceId" | "parser";
export function InvestigationStatistics({
  stats,
  sources,
  available,
  pending,
  disabled,
  onSelect,
}: {
  stats: Statistics;
  sources: SourceSummary[];
  available: boolean;
  pending: boolean;
  disabled: boolean;
  onSelect: (dimension: Dimension, value: string) => void;
}) {
  const groups: {
    title: string;
    dimension: Dimension;
    rows: GroupCount[];
    missing?: number;
  }[] = [
    {
      title: "Client IPs",
      dimension: "sourceIp",
      rows: stats.topIps,
      missing: stats.missingIp,
    },
    {
      title: "Request paths",
      dimension: "path",
      rows: stats.topPaths,
      missing: stats.missingPath,
    },
    { title: "Sources", dimension: "sourceId", rows: stats.sourceCounts },
    { title: "Readers", dimension: "parser", rows: stats.readerCounts },
  ];
  return (
    <section
      className="investigation-statistics"
      aria-label="Investigation breakdowns"
      aria-busy={pending}
    >
      <div className="statistics-section-heading">
        <h3>Investigation breakdowns</h3>
        <p>
          Top 10 per dimension · {available ? formatNumber(stats.total) : "…"}{" "}
          matching events across all pages. Select a value to inspect its logs.
        </p>
      </div>
      {!available ? (
        <p role="status">
          {pending
            ? "Updating investigation breakdowns…"
            : "Breakdowns unavailable. Retry the query."}
        </p>
      ) : (
        <div className="breakdown-grid">
          {groups.map((group) => (
            <section
              key={group.dimension}
              aria-label={group.title}
              className="breakdown-card"
            >
              <h4>{group.title}</h4>
              {group.rows.length ? (
                <ol>
                  {group.rows.map((item, index) => {
                    const label =
                      group.dimension === "sourceId"
                        ? `#${item.value} · ${sources.find((s) => String(s.id) === item.value)?.name ?? "Unknown source"}`
                        : item.value;
                    return (
                      <li key={index}>
                        <button
                          className="breakdown-value"
                          disabled={disabled || item.filterValue === null}
                          title={
                            item.filterValue ??
                            "Value exceeds the exact-filter limit of 4096 characters."
                          }
                          onClick={() =>
                            item.filterValue !== null &&
                            onSelect(group.dimension, item.filterValue)
                          }
                        >
                          <span>{label || "(empty string)"}</span>
                          <strong>{formatNumber(item.count)}</strong>
                        </button>
                      </li>
                    );
                  })}
                </ol>
              ) : (
                <p>No values in these results.</p>
              )}
              {group.missing !== undefined && (
                <p className="muted">
                  {formatNumber(group.missing)} events without this field
                </p>
              )}
            </section>
          ))}
        </div>
      )}
      <p className="muted">
        Counts include events without timestamps unless a time filter excludes
        them. Labels show at most 240 characters; values over 4096 characters
        cannot be used as exact filters.
      </p>
    </section>
  );
}
