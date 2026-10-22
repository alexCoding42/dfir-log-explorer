import { useEffect, useRef, useState } from "react";
import type { Bridge, CaseSummary, Query, SavedSearch } from "./types";

export function SavedSearches({
  bridge,
  info,
  query,
  disabled,
  onApply,
  onBusy,
  onError,
}: {
  bridge: Bridge;
  info: CaseSummary;
  query: Query;
  disabled: boolean;
  onApply: (query: Query) => void;
  onBusy: (busy: boolean) => void;
  onError: (message: string) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [searches, setSearches] = useState<SavedSearch[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [working, setWorking] = useState(false);
  const [ready, setReady] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  useEffect(() => {
    let current = true;
    setReady(false);
    setSearches([]);
    setSelected(null);
    setName("");
    setFilter("");
    setConfirmDelete(false);
    bridge
      .savedSearches()
      .then((items) => {
        if (current) {
          setSearches(items);
          setReady(true);
        }
      })
      .catch((reason) => {
        if (current) onError(String(reason));
      });
    return () => {
      current = false;
    };
  }, [bridge, info, onError]);
  useEffect(() => {
    if (!open) return;
    function outside(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    }
    function escape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    }
    document.addEventListener("mousedown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  async function run(action: "create" | "update" | "apply" | "delete") {
    if (working || disabled || !ready) return;
    setWorking(true);
    onBusy(true);
    try {
      if (action === "apply" && selected !== null) {
        const restored = await bridge.savedSearchQuery(selected);
        onApply(restored);
        setOpen(false);
      } else if (action === "delete" && selected !== null) {
        await bridge.deleteSavedSearch(selected);
        setSearches((items) => items.filter((s) => s.id !== selected));
        setSelected(null);
        setName("");
        setConfirmDelete(false);
      } else if (action === "create" || action === "update") {
        const result = await bridge.putSavedSearch(
          action === "create" ? null : selected,
          name,
          query,
        );
        setSearches((items) =>
          [...items.filter((s) => s.id !== result.id), result].sort(
            (a, b) => a.id - b.id,
          ),
        );
        setSelected(result.id);
        setName(result.name);
      }
    } catch (reason) {
      onError(String(reason));
    } finally {
      setWorking(false);
      onBusy(false);
    }
  }
  const locked = disabled || working || !ready;
  return (
    <div className="saved-search-control" ref={root}>
      <button
        ref={trigger}
        className="text-button"
        disabled={disabled}
        aria-expanded={open}
        aria-controls="saved-search-panel"
        onClick={() => setOpen(!open)}
      >
        Saved searches
      </button>
      {open && (
        <section
          id="saved-search-panel"
          className="saved-search-panel"
          aria-label="Saved searches"
        >
          <h3>Saved searches</h3>
          <p>
            {bridge.isDesktop
              ? info.path
                ? "Stored in this case."
                : "Temporary case: save the case to retain these searches."
              : "Synthetic preview: searches stay in memory until this page closes."}
          </p>
          <label>
            Find a saved search
            <input
              aria-label="Find a saved search"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              disabled={locked}
            />
          </label>
          <label>
            Saved search
            <select
              aria-label="Saved search"
              value={selected ?? ""}
              disabled={locked}
              onChange={(e) => {
                const id = e.target.value ? Number(e.target.value) : null;
                setSelected(id);
                setName(searches.find((s) => s.id === id)?.name ?? "");
                setConfirmDelete(false);
              }}
            >
              <option value="">New search</option>
              {searches
                .filter(
                  (s) =>
                    s.name.toLowerCase().includes(filter.toLowerCase()) ||
                    s.id === selected,
                )
                .map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
            </select>
          </label>
          {ready && searches.length === 0 && (
            <p>No saved searches in this case.</p>
          )}
          <label>
            Search name
            <input
              aria-label="Search name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={locked}
              maxLength={120}
            />
          </label>
          <p>
            Save the current filters and sort order. Update replaces the
            selected definition with the current query and name.
          </p>
          <div className="saved-search-actions">
            <button
              className="secondary-button"
              disabled={locked || !name.trim()}
              onClick={() => void run("create")}
            >
              Save as new
            </button>
            <button
              className="secondary-button"
              disabled={locked || selected === null || !name.trim()}
              onClick={() => void run("update")}
            >
              Update selected
            </button>
            <button
              className="primary-button"
              disabled={locked || selected === null}
              onClick={() => void run("apply")}
            >
              Apply search
            </button>
            <button
              className="text-button"
              disabled={locked || selected === null}
              onClick={() => setConfirmDelete(true)}
            >
              Delete search
            </button>
          </div>
          {confirmDelete && (
            <div
              className="delete-search-confirmation"
              role="group"
              aria-label="Confirm search deletion"
            >
              <p>Delete this saved search? Imported evidence is retained.</p>
              <button
                className="secondary-button"
                disabled={locked}
                onClick={() => void run("delete")}
              >
                Confirm delete search
              </button>
              <button
                className="text-button"
                disabled={locked}
                onClick={() => setConfirmDelete(false)}
              >
                Keep search
              </button>
            </div>
          )}
          <button className="text-button" onClick={() => setOpen(false)}>
            Close saved searches
          </button>
        </section>
      )}
    </div>
  );
}
