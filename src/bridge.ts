import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import type { Bridge, ImportProgress, SourceSummary } from "./types";

const desktopBridge: Bridge = {
  isDesktop: true,
  caseSummary: () => invoke("case_summary"),
  savedSearches: () => invoke("saved_searches"),
  putSavedSearch: (id, name, query) =>
    invoke("put_saved_search", { id, name, query }),
  deleteSavedSearch: (id) => invoke("delete_saved_search", { id }),
  savedSearchQuery: (id) => invoke("saved_search_query", { id }),
  saveCase: () => invoke("save_case"),
  openCase: () => invoke("open_case"),
  closeCase: () => invoke("close_case"),
  pickFile: () => invoke("pick_file"),
  importFile: (mode, handler) => {
    const onProgress = new Channel<ImportProgress>();
    onProgress.onmessage = handler;
    return invoke<SourceSummary>("import_file", { mode, onProgress });
  },
  importDemo: (handler) => {
    const onProgress = new Channel<ImportProgress>();
    onProgress.onmessage = handler;
    return invoke<SourceSummary>("import_demo", { onProgress });
  },
  cancelImport: () => invoke("cancel_import"),
  query: (query) => invoke("query_events", { query }),
  detail: (id) => invoke("event_detail", { id }),
};

export async function createBridge(): Promise<Bridge> {
  if (isTauri()) return desktopBridge;
  // Browser mode is deliberately limited to bundled synthetic events. Actual
  // sources can only be opened by the desktop's local Rust engine.
  const { previewBridge } = await import("./preview");
  return previewBridge;
}
