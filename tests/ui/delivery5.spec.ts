import { test, expect } from "@playwright/test";

// Tests the actual React -> Tauri command boundary with synthetic IPC only.
// Rust filesystem and recovery behavior is covered by volume.rs, not this mock.
test("checkpoint and export controls send the selected scope and raw consent to desktop commands", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: { cmd: string; args: unknown }[] = [];
    const source = {
      id: 1,
      name: "synthetic.log",
      provenance: "/synthetic/input.log",
      parserMode: "text",
      sha256: "a".repeat(64),
      totalBytes: 12,
      totalEvents: 2,
      parsedEvents: 0,
      partialEvents: 0,
      unparsedEvents: 2,
      textEvents: 2,
      warningEvents: 0,
      elapsedMs: 1,
    };
    let pending: unknown = {
      path: "/synthetic/large.log",
      events: 10000,
      bytesRead: 60000,
      totalBytes: 72000,
      mode: "text",
    };
    const callbacks = new Map<number, (p: unknown) => void>();
    let callbackId = 0;
    Object.assign(window, {
      isTauri: true,
      __volumeCalls: calls,
      __TAURI_INTERNALS__: {
        transformCallback: (fn: (p: unknown) => void) => {
          callbacks.set(++callbackId, fn);
          return callbackId;
        },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          calls.push({ cmd, args });
          if (cmd === "case_summary")
            return {
              name: "Synthetic saved case",
              path: "/synthetic/case.dfir",
              sources: [source],
            };
          if (cmd === "pending_import") return pending;
          if (cmd === "discard_pending_import") {
            pending = null;
            return;
          }
          if (cmd === "pick_file") return "large.log";
          if (cmd === "import_resumable") {
            pending = null;
            return source;
          }
          if (cmd === "export_results") {
            const channel = args.onProgress as { id: number };
            callbacks.get(channel.id)?.({
              index: 0,
              message: { eventsWritten: 600 },
            });
            return 600;
          }
          if (cmd === "query_events")
            return {
              rows: [],
              total: 0,
              offset: 0,
              limit: 500,
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
                missingIp: 0,
                missingPath: 0,
                topIps: [],
                topPaths: [],
                sourceCounts: [],
                readerCounts: [],
              },
            };
          if (["saved_searches", "parsing_profiles"].includes(cmd)) return [];
          throw new Error(`Unexpected mock command: ${cmd}`);
        },
      },
    });
  });
  await page.goto("/");
  await page.getByText("Ingestion and export", { exact: true }).click();
  await expect(page.getByText(/10,000 events checkpointed/)).toBeVisible();
  await page
    .getByRole("button", { name: "Resume import…", exact: true })
    .click();
  await expect(page.getByText(/Imported 2 events/)).toBeVisible();
  await page
    .getByRole("button", { name: "Export filtered results…", exact: true })
    .click();
  await expect(page.getByText(/Exported 600 matching events/)).toBeVisible();
  await page.getByLabel("Export format").selectOption("csv");
  await page.getByLabel("Include raw evidence").check();
  await page
    .getByRole("button", { name: "Export filtered results…", exact: true })
    .click();
  const exports = await page.evaluate(() =>
    (
      window as unknown as {
        __volumeCalls: {
          cmd: string;
          args: {
            includeRaw: boolean;
            format: string;
            query: { limit: number };
          };
        }[];
      }
    ).__volumeCalls
      .filter((c) => c.cmd === "export_results")
      .map((c) => c.args),
  );
  expect(exports.map((x) => [x.format, x.includeRaw])).toEqual([
    ["jsonLines", false],
    ["csv", true],
  ]);
  expect(exports[0].query.limit).toBe(500);
  await page.setViewportSize({ width: 1000, height: 720 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
