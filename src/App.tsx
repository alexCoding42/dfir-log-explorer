import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  ChevronLeft,
  ChevronRight,
  FileText,
  FlaskConical,
  FolderOpen,
  LoaderCircle,
  LockKeyhole,
  Monitor,
  Moon,
  Sun,
  X,
} from "lucide-react";
import { Sidebar, Topbar, PageHeading, ImportActions } from "./WorkspaceChrome";
import { EvidenceView, ParsersView, SettingsView } from "./WorkspaceViews";
import { LogSourceHeader } from "./LogSourceHeader";
import { CaseBar, SourceFilters } from "./CaseBar";
import { HelpDialog } from "./HelpDialog";
import type { View } from "./workspace";
import { DetailPanel } from "./DetailPanel";
import { EventTable } from "./EventTable";
import { ParsingSummary } from "./ParsingSummary";
import { TimelineChart } from "./TimelineChart";
import { LogFilterBar } from "./LogFilterBar";
import { formatBytes, formatNumber } from "./format";
import { usePreference, useTheme, type Theme } from "./preferences";
import type {
  Bridge,
  CaseSummary,
  JsonFilter,
  EventDetail,
  EventRow,
  FieldFilter,
  ImportProgress,
  ParserMode,
  ParseStatus,
  QueryPage,
  SearchField,
  SortField,
  SourceSummary,
} from "./types";
const PAGE_SIZE = 500;
const emptyPage: QueryPage = {
  rows: [],
  total: 0,
  offset: 0,
  limit: PAGE_SIZE,
  elapsedMs: 0,
  statistics: {
    total: 0,
    parsed: 0,
    partial: 0,
    unparsed: 0,
    warnings: 0,
    missingTimestamp: 0,
    totalRequests: 0,
    uniqueIps: 0,
    responses: [0, 0, 0, 0],
    timeline: [],
  },
};
export function App({ bridge }: { bridge: Bridge }) {
  const [caseInfo, setCaseInfo] = useState<CaseSummary>({
    name: "Temporary case",
    path: null,
    sources: [],
  });
  const [sourceId, setSourceId] = useState<number | null>(null);
  const [readerFilter, setReaderFilter] = useState<ParserMode | null>(null);
  const [hasTimestamp, setHasTimestamp] = useState<boolean | null>(null);
  const [jsonFilter, setJsonFilter] = useState<JsonFilter | null>(null);
  const source = useMemo<SourceSummary | null>(
    () =>
      sourceId !== null
        ? (caseInfo.sources.find((s) => s.id === sourceId) ?? null)
        : caseInfo.sources.length === 1
          ? caseInfo.sources[0]
          : caseInfo.sources.length === 0
            ? null
            : {
                ...caseInfo.sources[0],
                id: 0,
                name: `${caseInfo.name} · ${caseInfo.sources.length} sources`,
                provenance: caseInfo.path ?? "Temporary local case",
                parserMode: "auto",
                sha256: "",
                ...Object.fromEntries(
                  [
                    "totalBytes",
                    "totalEvents",
                    "parsedEvents",
                    "partialEvents",
                    "unparsedEvents",
                    "textEvents",
                    "warningEvents",
                    "elapsedMs",
                  ].map((key) => [
                    key,
                    caseInfo.sources.reduce(
                      (sum, item) =>
                        sum + Number(item[key as keyof SourceSummary]),
                      0,
                    ),
                  ]),
                ),
              },
    [caseInfo, sourceId],
  );

  const [page, setPage] = useState<QueryPage>(emptyPage);
  const [mode, setMode] = useState<ParserMode>("auto");
  const [view, setView] = useState<View>("Explore");
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [scope, setScope] = useState<SearchField>("all");
  const [onlyWarnings, setOnlyWarnings] = useState(false);
  const [parseStatus, setParseStatus] = useState<ParseStatus | null>(null);
  const [fieldFilter, setFieldFilter] = useState<FieldFilter | null>(null);
  const [timeRange, setTimeRange] = useState<[number | null, number | null]>([
    null,
    null,
  ]);
  const [sortBy, setSortBy] = useState<SortField>("lineNumber");
  const [descending, setDescending] = useState(false);
  const [offset, setOffset] = useState(0);
  const [detail, setDetail] = useState<EventDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [pageAvailable, setPageAvailable] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [picking, setPicking] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [importName, setImportName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [help, setHelp] = useState(false);
  const [theme, setTheme] = useTheme();
  const [collapsed, setCollapsed] = usePreference<boolean>(
    "sidebarCollapsed",
    true,
    (v): v is boolean => typeof v === "boolean",
  );
  const searchInput = useRef<HTMLInputElement>(null);
  const detailRequest = useRef(0);
  const importRunning = useRef(false);
  const disabled = busy || picking;
  const closeDetails = useCallback(() => {
    detailRequest.current++;
    setDetail(null);
    setDetailLoading(false);
  }, []);
  const resetFilters = useCallback(() => {
    setSourceId(null);
    setReaderFilter(null);
    setHasTimestamp(null);
    setJsonFilter(null);
    setSearch("");
    setDebouncedSearch("");
    setOnlyWarnings(false);
    setParseStatus(null);
    setScope("all");
    setFieldFilter(null);
    setTimeRange([null, null]);
    setOffset(0);
  }, []);
  useEffect(() => {
    const timeout = setTimeout(() => {
      setOffset(0);
      setDebouncedSearch(search);
    }, 250);
    return () => clearTimeout(timeout);
  }, [search]);
  useEffect(() => {
    closeDetails();
  }, [
    source,
    sourceId,
    readerFilter,
    hasTimestamp,
    jsonFilter,
    debouncedSearch,
    scope,
    onlyWarnings,
    parseStatus,
    fieldFilter,
    timeRange,
    sortBy,
    descending,
    offset,
    closeDetails,
  ]);
  useEffect(() => {
    if (!source || busy || picking) return;
    let current = true;
    setLoading(true);
    bridge
      .query({
        sourceId,
        parser: readerFilter,
        hasTimestamp,
        jsonFilter,
        search: debouncedSearch,
        searchField: scope,
        onlyWarnings,
        parseStatus,
        fieldFilter,
        timeFrom: timeRange[0],
        timeTo: timeRange[1],
        sortBy,
        descending,
        offset,
        limit: PAGE_SIZE,
      })
      .then((result) => {
        if (current) {
          setPage(result);
          setPageAvailable(true);
          setLoading(false);
        }
      })
      .catch((reason: unknown) => {
        if (current) {
          setError(String(reason));
          setPage(emptyPage);
          setPageAvailable(false);
          setLoading(false);
        }
      });
    return () => {
      current = false;
    };
  }, [
    bridge,
    source,
    sourceId,
    readerFilter,
    hasTimestamp,
    jsonFilter,
    debouncedSearch,
    scope,
    onlyWarnings,
    parseStatus,
    fieldFilter,
    timeRange,
    sortBy,
    descending,
    offset,
    busy,
    picking,
  ]);
  const beginImport = useCallback(
    async (demo = false) => {
      if (importRunning.current) return;
      importRunning.current = true;
      setError(null);
      setNotice(null);
      try {
        if (!demo) {
          setPicking(true);
          const name = await bridge.pickFile();
          setPicking(false);
          if (!name) return;
          setImportName(name);
        } else {
          setImportName("apache-mixed.log · synthetic sample");
        }
        closeDetails();
        setBusy(true);
        setProgress(null);
        setCancelling(false);
        const summary = await (demo
          ? bridge.importDemo(setProgress)
          : bridge.importFile(mode, setProgress));
        setCaseInfo(await bridge.caseSummary());
        setView("Explore");
        resetFilters();
        setSortBy("lineNumber");
        setDescending(false);
        setPage(emptyPage);
        setPageAvailable(false);
        setNotice(
          `Imported ${formatNumber(summary.totalEvents)} events from ${formatNumber(summary.totalLines || summary.totalEvents)} lines. Source file unchanged.`,
        );
      } catch (reason) {
        const message =
          reason instanceof Error ? reason.message : String(reason);
        if (message.toLowerCase().includes("cancelled")) setNotice(message);
        else setError(message);
      } finally {
        importRunning.current = false;
        setBusy(false);
        setPicking(false);
        setCancelling(false);
        setProgress(null);
      }
    },
    [bridge, mode, closeDetails, resetFilters],
  );
  useEffect(() => {
    let current = true;
    bridge
      .caseSummary()
      .then((info) => {
        if (current) setCaseInfo(info);
      })
      .catch((reason) => {
        if (current) setError(String(reason));
      });
    return () => {
      current = false;
    };
  }, [bridge]);
  async function changeCase(action: "save" | "open" | "new") {
    if (importRunning.current) return;
    if (
      bridge.isDesktop &&
      action !== "save" &&
      !caseInfo.path &&
      caseInfo.sources.length > 0
    ) {
      setError(
        "Save this temporary case before switching so your imported evidence is retained.",
      );
      return;
    }
    importRunning.current = true;
    setPicking(true);
    setError(null);
    try {
      const info = await (action === "save"
        ? bridge.saveCase()
        : action === "open"
          ? bridge.openCase()
          : bridge.closeCase());
      if (info) {
        closeDetails();
        setCaseInfo(info);
        resetFilters();
        setPage(emptyPage);
        setPageAvailable(false);
        setSortBy("lineNumber");
        setDescending(false);
        setView("Explore");
        setNotice(
          action === "save"
            ? "Case saved locally. Future imports are saved automatically."
            : "Case ready.",
        );
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      importRunning.current = false;
      setPicking(false);
    }
  }
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setView("Explore");
        requestAnimationFrame(() => searchInput.current?.focus());
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "o") {
        event.preventDefault();
        if (!disabled) void beginImport();
      }
      if (event.key === "Escape") {
        closeDetails();
        setHelp(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [beginImport, disabled, closeDetails]);
  useEffect(() => {
    if (!help) return;
    const previous = document.activeElement as HTMLElement | null;
    const dialog = document.querySelector<HTMLElement>(".modal");
    if (!dialog) return;
    dialog.querySelector<HTMLElement>("button")?.focus();
    const trap = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const controls = Array.from(
        dialog.querySelectorAll<HTMLElement>(
          "button, a, input, select, [tabindex='0']",
        ),
      );
      const first = controls[0],
        last = controls.at(-1);
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    dialog.addEventListener("keydown", trap);
    return () => {
      dialog.removeEventListener("keydown", trap);
      previous?.focus();
    };
  }, [help]);
  async function selectEvent(row: EventRow) {
    if (busy || loading) return;
    const request = ++detailRequest.current;
    setDetailLoading(true);
    try {
      const selected = await bridge.detail(row.id);
      if (request === detailRequest.current) setDetail(selected);
    } catch (reason) {
      if (request === detailRequest.current) setError(String(reason));
    } finally {
      if (request === detailRequest.current) setDetailLoading(false);
    }
  }
  async function cancel() {
    setCancelling(true);
    try {
      await bridge.cancelImport();
    } catch (reason) {
      setError(String(reason));
      setCancelling(false);
    }
  }
  function statusFilter(status: ParseStatus | null) {
    setParseStatus(status);
    setOffset(0);
    setView("Explore");
  }
  function exactFilter(filter: FieldFilter | null) {
    setFieldFilter(filter);
    setOffset(0);
  }
  function rangeFilter(range: [number | null, number | null]) {
    setTimeRange(range);
    setOffset(0);
  }
  const filtered =
    sourceId !== null ||
    readerFilter !== null ||
    hasTimestamp !== null ||
    jsonFilter !== null ||
    debouncedSearch.length > 0 ||
    onlyWarnings ||
    parseStatus !== null ||
    fieldFilter !== null ||
    timeRange.some((v) => v !== null);
  const percent =
    progress && progress.totalBytes > 0
      ? Math.min(100, (progress.bytesRead / progress.totalBytes) * 100)
      : 0;
  const rangeStart = page.total === 0 ? 0 : offset + 1;
  const rangeEnd = Math.min(offset + page.rows.length, page.total);
  const selectedIndex = page.rows.findIndex((row) => row.id === detail?.id);
  const ThemeIcon =
    theme === "dark" ? Moon : theme === "system" ? Monitor : Sun;
  const themeControl = (
    <label className="theme-control">
      <ThemeIcon size={15} />
      <select
        aria-label="Theme preference"
        value={theme}
        onChange={(e) => setTheme(e.target.value as Theme)}
      >
        <option value="light">Light</option>
        <option value="dark">Dark</option>
        <option value="system">System</option>
      </select>
    </label>
  );
  function navigate(next: View) {
    setView(next);
    closeDetails();
  }
  return (
    <div className={`app-layout ${collapsed ? "sidebar-collapsed" : ""}`}>
      <Sidebar
        source={source}
        view={view}
        navigate={navigate}
        disabled={disabled}
        onImport={() => void beginImport()}
        collapsed={collapsed}
        onToggle={() => setCollapsed(!collapsed)}
        onHelp={() => setHelp(true)}
      />
      <main className="main-content">
        <Topbar
          caseName={caseInfo.name}
          source={source}
          busy={disabled}
          search={search}
          setSearch={setSearch}
          searchInput={searchInput}
          navigate={navigate}
          themeControl={themeControl}
          caseControl={
            <CaseBar
              info={caseInfo}
              disabled={disabled}
              onSave={() => void changeCase("save")}
              onOpen={() => void changeCase("open")}
              onNew={() => void changeCase("new")}
            />
          }
          importControls={
            <ImportActions
              mode={mode}
              setMode={setMode}
              disabled={disabled}
              picking={picking}
              beginImport={beginImport}
            />
          }
        />
        {!bridge.isDesktop && (
          <div className="preview-banner">
            <FlaskConical size={13} />
            <span>
              Interface preview · synthetic data only. Open the desktop app to
              analyze your files.
            </span>
          </div>
        )}
        {view !== "Explore" && view !== "Statistics" && (
          <PageHeading view={view} />
        )}
        {error && (
          <div role="alert" className="message error-message">
            <AlertTriangle size={16} />
            <span>{error}</span>
            <button
              className="icon-button"
              aria-label="Dismiss error"
              onClick={() => setError(null)}
            >
              <X size={15} />
            </button>
          </div>
        )}
        {busy && (
          <div className="import-progress" role="status">
            <div className="progress-heading">
              <LoaderCircle size={16} className="spin" />
              <strong>
                {cancelling ? "Cancelling import…" : "Importing locally…"}
              </strong>
              <span>
                {importName} · {formatNumber(progress?.linesRead ?? 0)} lines
                read · {formatBytes(progress?.bytesRead ?? 0)} /{" "}
                {formatBytes(progress?.totalBytes ?? 0)}
              </span>
              <button
                className="secondary-button"
                disabled={cancelling}
                onClick={() => void cancel()}
              >
                Cancel import
              </button>
            </div>
            <div
              className="progress-track"
              role="progressbar"
              aria-label="Import progress"
              aria-valuenow={Math.round(percent)}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <div style={{ width: `${percent}%` }} />
            </div>
          </div>
        )}
        {view === "Explore" &&
          (source ? (
            <div className="explorer-workspace">
              <div className="explorer-main">
                <section className="explorer-card">
                  <LogFilterBar
                    source={source}
                    sourceSelector={
                      <label className="source-selector">
                        <span>Source</span>
                        <select
                          aria-label="Source filter"
                          disabled={disabled}
                          value={sourceId ?? ""}
                          onChange={(e) => {
                            setSourceId(
                              e.target.value ? Number(e.target.value) : null,
                            );
                            setOffset(0);
                          }}
                        >
                          <option value="">
                            All sources ({caseInfo.sources.length})
                          </option>
                          {caseInfo.sources.map((item) => (
                            <option value={item.id} key={item.id}>
                              #{item.id} · {item.name}
                            </option>
                          ))}
                        </select>
                      </label>
                    }
                    advancedActive={
                      readerFilter !== null ||
                      hasTimestamp !== null ||
                      jsonFilter !== null ||
                      fieldFilter !== null ||
                      scope !== "all" ||
                      timeRange.some((value) => value !== null)
                    }
                    advanced={
                      <SourceFilters
                        parser={readerFilter}
                        onParser={(value) => {
                          setReaderFilter(value);
                          setOffset(0);
                        }}
                        hasTimestamp={hasTimestamp}
                        onTimestamp={(value) => {
                          setHasTimestamp(value);
                          setOffset(0);
                        }}
                        jsonFilter={jsonFilter}
                        onJson={(value) => {
                          setJsonFilter(value);
                          setOffset(0);
                        }}
                        disabled={disabled}
                      />
                    }
                    extraChips={
                      (readerFilter !== null ||
                        hasTimestamp !== null ||
                        jsonFilter !== null) && (
                        <div className="filter-chips">
                          {readerFilter && (
                            <button
                              className="filter-chip"
                              disabled={disabled}
                              onClick={() => {
                                setReaderFilter(null);
                                setOffset(0);
                              }}
                            >
                              Reader: {readerFilter.replaceAll("_", " ")} ×
                            </button>
                          )}
                          {hasTimestamp !== null && (
                            <button
                              className="filter-chip"
                              disabled={disabled}
                              onClick={() => {
                                setHasTimestamp(null);
                                setOffset(0);
                              }}
                            >
                              Timestamp:{" "}
                              {hasTimestamp ? "valid" : "missing / invalid"} ×
                            </button>
                          )}
                          {jsonFilter && (
                            <button
                              className="filter-chip"
                              disabled={disabled}
                              onClick={() => {
                                setJsonFilter(null);
                                setOffset(0);
                              }}
                            >
                              {jsonFilter.key} = {jsonFilter.value} ×
                            </button>
                          )}
                        </div>
                      )
                    }
                    status={parseStatus}
                    onStatus={statusFilter}
                    warnings={onlyWarnings}
                    onWarnings={() => {
                      setOnlyWarnings(!onlyWarnings);
                      setOffset(0);
                    }}
                    warningCount={source.warningEvents}
                    scope={scope}
                    onScope={(v) => {
                      setScope(v);
                      setOffset(0);
                    }}
                    fieldFilter={fieldFilter}
                    onFieldFilter={exactFilter}
                    timeRange={timeRange}
                    onTimeRange={rangeFilter}
                    onReset={resetFilters}
                    filtered={filtered}
                    disabled={busy}
                    onError={setError}
                  />
                  <EventTable
                    sources={caseInfo.sources}
                    rows={page.rows}
                    selectedId={detail?.id ?? null}
                    onSelect={(row) => void selectEvent(row)}
                    loading={loading || busy}
                    filtered={filtered}
                    sortBy={sortBy}
                    descending={descending}
                    onSort={(key) => {
                      setDescending(sortBy === key ? !descending : false);
                      setSortBy(key);
                      setOffset(0);
                    }}
                  />
                  <footer className="table-footer">
                    <span>
                      {formatNumber(rangeStart)}–{formatNumber(rangeEnd)} of{" "}
                      {formatNumber(page.total)} events
                    </span>
                    <div>
                      <span className="query-time">
                        {detailLoading
                          ? "Loading event…"
                          : bridge.isDesktop
                            ? `${page.elapsedMs} ms`
                            : "Synthetic preview"}
                      </span>
                      <span>Page {Math.floor(offset / PAGE_SIZE) + 1}</span>
                      <button
                        className="icon-button"
                        aria-label="Previous page"
                        disabled={offset === 0 || loading || busy}
                        onClick={() =>
                          setOffset(Math.max(0, offset - PAGE_SIZE))
                        }
                      >
                        <ChevronLeft size={16} />
                      </button>
                      <button
                        className="icon-button"
                        aria-label="Next page"
                        disabled={
                          offset + PAGE_SIZE >= page.total || loading || busy
                        }
                        onClick={() => setOffset(offset + PAGE_SIZE)}
                      >
                        <ChevronRight size={16} />
                      </button>
                    </div>
                  </footer>
                </section>
              </div>
              {detail && (
                <DetailPanel
                  detail={detail}
                  source={
                    caseInfo.sources.find((s) => s.id === detail.sourceId) ??
                    source
                  }
                  onJsonFilter={(filter) => {
                    setJsonFilter(filter);
                    setOffset(0);
                  }}
                  onClose={closeDetails}
                  onFilter={exactFilter}
                  onParsingFilter={statusFilter}
                  onNavigate={(direction) => {
                    const row = page.rows[selectedIndex + direction];
                    if (row) void selectEvent(row);
                  }}
                  hasPrevious={selectedIndex > 0}
                  hasNext={
                    selectedIndex >= 0 && selectedIndex < page.rows.length - 1
                  }
                  disabled={busy || loading || detailLoading}
                />
              )}
            </div>
          ) : (
            !busy && (
              <div className="empty-state">
                <div className="empty-icon">
                  <FileText size={32} strokeWidth={1.4} />
                </div>
                <span className="eyebrow">START WITH YOUR SOURCE</span>
                <h2>A clearer view of your logs.</h2>
                <p>
                  Import, parse, and explore every line.
                  <br />
                  Keep the original evidence within reach.
                </p>
                <button
                  className="primary-button"
                  disabled={disabled}
                  onClick={() => void beginImport()}
                >
                  <FolderOpen size={16} />
                  Choose a log file
                  <ArrowRight size={15} />
                </button>
                <button
                  className="sample-button"
                  disabled={disabled}
                  onClick={() => void beginImport(true)}
                >
                  <FlaskConical size={14} />
                  Explore a synthetic sample
                  <ArrowRight size={14} />
                </button>
                <div className="supported-formats">
                  Apache / Nginx <span>·</span> JSON Lines <span>·</span> Syslog
                  <span>·</span> Firewall / WAF <span>·</span> HAProxy
                </div>
                <div className="empty-workflow">
                  <span>
                    <i>01</i>Import a source
                  </span>
                  <span>
                    <i>02</i>Review parsing
                  </span>
                  <span>
                    <i>03</i>Inspect events
                  </span>
                </div>
              </div>
            )
          ))}
        {view === "Statistics" && (
          <section className="secondary-view statistics-view">
            <div className="statistics-heading">
              <div>
                <h1>Statistics</h1>
                <p>Review log volume, parsing quality and HTTP activity.</p>
              </div>
              <button
                className="secondary-button"
                onClick={() => navigate("Explore")}
              >
                <ChevronLeft size={14} />
                Back to logs
              </button>
            </div>
            {source ? (
              <>
                <LogSourceHeader
                  source={source}
                  onEvidence={() => navigate("Evidence")}
                />
                <ParsingSummary
                  source={source}
                  disabled={busy}
                  stats={page.statistics}
                  available={pageAvailable && !loading && !busy}
                  pending={loading || busy}
                  filtered={filtered}
                  active={parseStatus}
                  onFilter={statusFilter}
                />
                <TimelineChart
                  stats={page.statistics}
                  filtered={filtered}
                  available={pageAvailable && !loading && !busy}
                  pending={loading || busy}
                  disabled={busy || loading}
                  onRange={(from, to) => {
                    rangeFilter([from, to]);
                    setView("Explore");
                  }}
                />
              </>
            ) : (
              <p>Import a source to view its statistics.</p>
            )}
          </section>
        )}
        {view === "Evidence" && (
          <EvidenceView
            sources={caseInfo.sources}
            onExplore={() => navigate("Explore")}
          />
        )}
        {view === "Parsers" && (
          <ParsersView
            mode={mode}
            disabled={disabled}
            onSelect={(mode) => {
              setMode(mode);
              navigate("Explore");
            }}
          />
        )}
        {view === "Settings" && (
          <SettingsView
            themeControl={themeControl}
            collapsed={collapsed}
            onToggle={() => setCollapsed(!collapsed)}
            onHelp={() => setHelp(true)}
          />
        )}
        <footer className="workspace-footer">
          <span>
            <span className="live-dot" />
            {caseInfo.path ? "Saved case" : "Temporary case"}{" "}
            <span className="footer-separator">/</span>
            <LockKeyhole size={11} />
            Local processing
          </span>
          {notice && !busy ? (
            <span role="status" className="footer-notice">
              {notice}
              <button
                className="icon-button"
                aria-label="Dismiss notification"
                onClick={() => setNotice(null)}
              >
                <X size={12} />
              </button>
            </span>
          ) : (
            <span>
              DFIR Log Explorer <span className="footer-version">v0.3.0</span>
            </span>
          )}
        </footer>
      </main>
      {help && <HelpDialog onClose={() => setHelp(false)} />}
    </div>
  );
}
