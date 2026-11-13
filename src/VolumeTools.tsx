import { useEffect, useState } from "react";
import type { Bridge, CaseSummary, PendingImport, Query } from "./types";

export function VolumeTools({
  bridge,
  info,
  query,
  disabled,
  onStart,
  onBusy,
  onError,
  onNotice,
}: {
  bridge: Bridge;
  info: CaseSummary;
  query: Query;
  disabled: boolean;
  onStart: () => void;
  onBusy: (busy: boolean) => void;
  onError: (error: string | null) => void;
  onNotice: (notice: string | null) => void;
}) {
  const [pending, setPending] = useState<PendingImport | null>(null);
  const [format, setFormat] = useState<"csv" | "jsonLines">("jsonLines");
  const [raw, setRaw] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [count, setCount] = useState(0);
  useEffect(() => {
    let current = true;
    if (!disabled && bridge.pendingImport) {
      bridge
        .pendingImport()
        .then((p) => {
          if (current) setPending(p);
        })
        .catch((e) => {
          if (current) onError(String(e));
        });
    }
    return () => {
      current = false;
    };
  }, [bridge, info, disabled, onError]);
  if (!bridge.isDesktop || !bridge.pendingImport) return null;
  async function discard() {
    onBusy(true);
    onError(null);
    try {
      await bridge.discardPendingImport?.();
      setPending(null);
      onNotice("Incomplete import discarded. Completed sources unchanged.");
    } catch (e) {
      onError(String(e));
    } finally {
      onBusy(false);
    }
  }
  async function exportResults() {
    if (!bridge.exportResults) return;
    onBusy(true);
    setExporting(true);
    setCount(0);
    onError(null);
    try {
      const written = await bridge.exportResults(query, format, raw, setCount);
      if (written !== null)
        onNotice(
          `Exported ${written.toLocaleString()} matching events. Source evidence unchanged.`,
        );
    } catch (e) {
      onError(String(e));
    } finally {
      setExporting(false);
      onBusy(false);
    }
  }
  return (
    <section className="volume-tools" aria-label="Ingestion and export">
      <details>
        <summary>Ingestion and export</summary>
        <p>
          Gzip sources are supported up to 10 GiB decoded. Save the case to
          enable resumable imports. Completed batches survive interruption;
          incomplete sources stay hidden.
        </p>
        {pending && (
          <p role="status">
            Paused: {pending.path} · {pending.events.toLocaleString()} events
            checkpointed. Select the original unchanged file to resume.
          </p>
        )}
        <div className="volume-actions">
          <button
            className="secondary-button"
            disabled={disabled || !info.path}
            onClick={onStart}
          >
            {pending ? "Resume import…" : "Import with checkpoints…"}
          </button>
          {info.path && (
            <button
              className="secondary-button"
              disabled={disabled}
              onClick={() => void discard()}
            >
              Discard incomplete import
            </button>
          )}
        </div>
        <p>
          Export all filtered results in the selected order, across all pages.
          Files are unencrypted and include source paths and provenance. Raw
          content is optional. CSV stores structured fields in JSON cells.
        </p>
        <div className="volume-actions">
          <label>
            Export format{" "}
            <select
              aria-label="Export format"
              value={format}
              disabled={disabled}
              onChange={(e) => setFormat(e.target.value as typeof format)}
            >
              <option value="jsonLines">JSON Lines</option>
              <option value="csv">CSV</option>
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={raw}
              disabled={disabled}
              onChange={(e) => setRaw(e.target.checked)}
            />{" "}
            Include raw evidence
          </label>
          <button
            className="secondary-button"
            disabled={disabled || !info.sources.length}
            onClick={() => void exportResults()}
          >
            Export filtered results…
          </button>
        </div>
        {exporting && (
          <div role="status">
            Exporting · {count.toLocaleString()} events written{" "}
            <button
              className="secondary-button"
              onClick={() =>
                void bridge.cancelImport().catch((e) => onError(String(e)))
              }
            >
              Cancel export
            </button>
          </div>
        )}
      </details>
    </section>
  );
}
