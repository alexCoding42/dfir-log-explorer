import { X } from "lucide-react";
export function HelpDialog({ onClose }: { onClose: () => void }) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label="Application help"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-heading">
          <h2>A private workspace for your logs</h2>
          <button
            className="icon-button"
            aria-label="Close dialog"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
        <p>
          Choose a local file and a preconfigured parser. Every source line
          remains available, including Partial and Unparsed results.
        </p>
        <p>
          Sources are opened read-only. Timestamps preserve their source
          offsets; timeline intervals and time filters use UTC explicitly.
          Missing timestamps stay outside the timeline. BSD Syslog and standard
          HAProxy dates have no known UTC instant and retain ambiguity
          diagnostics.
        </p>
        <p>
          Imports add sources to the active case; cancellation preserves
          previous evidence. Save case as… retains all imported data in a local
          file. Saved cases reopen without the original logs and save future
          imports automatically. Unsaved temporary cases are removed on normal
          exit.
        </p>
        <p>
          Select security readers explicitly. ModSecurity audit groups native
          2.x A–Z transactions; the inspector shows source-line ranges and
          reader attributes. Exact attribute filters are available from the
          inspector or Filters. Multiline events are limited to 1 MiB; exceeding
          the limit rolls back the complete import.
        </p>
        <div className="shortcut-list">
          <span>
            <kbd>Ctrl O</kbd>Import a file
          </span>
          <span>
            <kbd>Ctrl K</kbd>Focus search
          </span>
          <span>
            <kbd>↑ ↓</kbd>Navigate visible event rows
          </span>
          <span>
            <kbd>Enter</kbd>Inspect event
          </span>
          <span>
            <kbd>Esc</kbd>Close inspector or help
          </span>
        </div>
      </section>
    </div>
  );
}
