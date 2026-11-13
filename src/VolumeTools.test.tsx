// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, it, expect, vi } from "vitest";
import { VolumeTools } from "./VolumeTools";
import type { Bridge, CaseSummary, Query } from "./types";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

describe("ingestion and export controls", () => {
  it("exports the current filter with explicit raw consent and exposes retained checkpoint actions", async () => {
    const pendingImport = vi
      .fn()
      .mockResolvedValue({ path: "/synthetic/input.log", events: 10_000 });
    const discardPendingImport = vi.fn().mockResolvedValue(undefined);
    const exportResults = vi
      .fn()
      .mockImplementation(async (_q, _f, _r, progress) => {
        progress(600);
        return 600;
      });
    const bridge = {
      isDesktop: true,
      pendingImport,
      discardPendingImport,
      exportResults,
    } as unknown as Bridge;
    const info = {
      name: "Synthetic case",
      path: "/synthetic/case.dfir",
      sources: [{}],
    } as CaseSummary;
    const query = { search: "login", offset: 500, limit: 500 } as Query;
    const notice = vi.fn(),
      start = vi.fn(),
      busy = vi.fn(),
      error = vi.fn();
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    try {
      await act(async () =>
        root.render(
          <VolumeTools
            bridge={bridge}
            info={info}
            query={query}
            disabled={false}
            onStart={start}
            onBusy={busy}
            onError={error}
            onNotice={notice}
          />,
        ),
      );
      const buttons = () => Array.from(container.querySelectorAll("button"));
      expect(container.textContent).toContain("10,000 events checkpointed");
      await act(async () =>
        buttons()
          .find((b) => b.textContent === "Resume import…")!
          .click(),
      );
      expect(start).toHaveBeenCalledOnce();
      await act(async () =>
        buttons()
          .find((b) => b.textContent === "Export filtered results…")!
          .click(),
      );
      expect(exportResults).toHaveBeenLastCalledWith(
        query,
        "jsonLines",
        false,
        expect.any(Function),
      );
      const raw = container.querySelector<HTMLInputElement>(
        'input[type="checkbox"]',
      )!;
      await act(async () => raw.click());
      await act(async () =>
        buttons()
          .find((b) => b.textContent === "Export filtered results…")!
          .click(),
      );
      expect(exportResults).toHaveBeenLastCalledWith(
        query,
        "jsonLines",
        true,
        expect.any(Function),
      );
      await act(async () =>
        buttons()
          .find((b) => b.textContent === "Discard incomplete import")!
          .click(),
      );
      expect(discardPendingImport).toHaveBeenCalledOnce();
      expect(container.textContent).not.toContain("events checkpointed");
      expect(busy).toHaveBeenLastCalledWith(false);
      expect(error.mock.calls.every(([e]) => e === null)).toBe(true);
    } finally {
      await act(async () => root.unmount());
      container.remove();
    }
  });
});
