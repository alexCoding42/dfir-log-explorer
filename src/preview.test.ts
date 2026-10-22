import { describe, expect, it } from "vitest";
import { previewBridge } from "./preview";

describe("browser preview data boundary", () => {
  it("refuses real file imports and exposes only the synthetic sample", async () => {
    await expect(previewBridge.pickFile()).rejects.toThrow(
      "synthetic data only",
    );
    await expect(previewBridge.importFile("auto", () => {})).rejects.toThrow(
      "desktop application",
    );
    await previewBridge.importDemo(() => {});
    const result = await previewBridge.query({
      search: "/login",
      onlyWarnings: false,
      offset: 0,
      limit: 500,
    });
    expect(result.total).toBe(2);
    expect(result.rows.every((row) => row.sourceIp === "198.51.100.24")).toBe(
      true,
    );
  });
});

it("filters, sorts and aggregates the complete synthetic source consistently", async () => {
  await previewBridge.importDemo(() => {});
  const base = { search: "", onlyWarnings: false, offset: 0, limit: 2 };
  const all = await previewBridge.query(base);
  expect(all.rows).toHaveLength(2);
  expect(all.statistics).toMatchObject({
    total: 13,
    parsed: 11,
    partial: 1,
    unparsed: 1,
    warnings: 1,
    missingTimestamp: 2,
    totalRequests: 12,
    uniqueIps: 8,
    responses: [7, 0, 4, 1],
  });
  expect(
    all.statistics.timeline.reduce(
      (sum, b) => sum + b.parsed + b.partial + b.unparsed,
      0,
    ),
  ).toBe(11);
  const partial = await previewBridge.query({
    ...base,
    parseStatus: "partial",
    onlyWarnings: true,
  });
  expect(partial.total).toBe(1);
  expect(partial.rows[0]).toMatchObject({
    sourceIp: "192.0.2.14",
    timestamp: null,
    status: 200,
  });
  expect(partial.statistics.timeline).toEqual([]);
  const unknown = await previewBridge.query({
    ...base,
    parseStatus: "unparsed",
    search: "<img src=",
    searchField: "raw",
  });
  expect(unknown.total).toBe(1);
  expect((await previewBridge.detail(unknown.rows[0].id)).raw).toContain(
    "<img src=",
  );
  const exact = await previewBridge.query({
    ...base,
    fieldFilter: { field: "sourceIp", value: "198.51.100.24" },
  });
  expect(exact.total).toBe(3);
  expect(exact.statistics.responses).toEqual([0, 0, 3, 0]);
  expect(
    (await previewBridge.query({ ...base, sortBy: "status", descending: true }))
      .rows[0].status,
  ).toBe(502);
  expect(
    (
      await previewBridge.query({
        ...base,
        searchField: "timestamp",
        search: "2026-10-13T09:14:00",
      })
    ).total,
  ).toBe(1);
  const start = (await previewBridge.detail(1)).timestampMs!;
  expect(
    (
      await previewBridge.query({
        ...base,
        timeFrom: start,
        timeTo: start + 1000,
      })
    ).total,
  ).toBe(1);
  await expect(
    previewBridge.query({ ...base, timeFrom: 2, timeTo: 1 }),
  ).rejects.toThrow("Invalid query");
});

it("keeps persistent cases behind the native bridge and combines source/date/parser filters", async () => {
  await previewBridge.closeCase();
  expect((await previewBridge.caseSummary()).sources).toEqual([]);
  await expect(previewBridge.saveCase()).rejects.toThrow("desktop application");
  await expect(previewBridge.openCase()).rejects.toThrow("desktop application");
  await previewBridge.importDemo(() => {});
  expect((await previewBridge.caseSummary()).sources[0].id).toBe(1);
  const base = { search: "", onlyWarnings: false, offset: 0, limit: 500 };
  expect((await previewBridge.query({ ...base, sourceId: 999 })).total).toBe(0);
  expect(
    (
      await previewBridge.query({
        ...base,
        sourceId: 1,
        parser: "apache",
        hasTimestamp: false,
      })
    ).total,
  ).toBe(1);
  expect((await previewBridge.query({ ...base, parser: "nginx" })).total).toBe(
    0,
  );
  await previewBridge.closeCase();
  expect((await previewBridge.query(base)).total).toBe(0);
});

it("counts complete breakdowns and adds exact drill-down filters", async () => {
  await previewBridge.closeCase();
  await previewBridge.importDemo(() => {});
  const base = { search: "", onlyWarnings: false, offset: 0, limit: 1 };
  const page = await previewBridge.query(base);
  expect(page.statistics.topIps[0]).toMatchObject({
    value: "198.51.100.24",
    count: 3,
  });
  expect(page.statistics.sourceCounts).toEqual([
    { value: "1", count: 13, filterValue: "1" },
  ]);
  expect(page.statistics.missingIp).toBe(1);
  for (const group of page.statistics.topIps) {
    const scoped = await previewBridge.query({
      ...base,
      fieldFilters: [{ field: "sourceIp", value: group.filterValue! }],
    });
    expect(scoped.total).toBe(group.count);
  }
  const and = await previewBridge.query({
    ...base,
    fieldFilter: { field: "method", value: "POST" },
    fieldFilters: [{ field: "sourceIp", value: "198.51.100.24" }],
  });
  expect(and.total).toBe(2);
});

it("keeps preview saved searches in memory, restores parameters, and clears them with the case", async () => {
  await previewBridge.closeCase();
  await previewBridge.importDemo(() => {});
  const query = {
    search: "login",
    searchField: "path" as const,
    onlyWarnings: false,
    offset: 100,
    limit: 1,
    sortBy: "status" as const,
    descending: true,
    sourceId: 1,
    fieldFilters: [{ field: "method" as const, value: "POST" }],
  };
  const saved = await previewBridge.putSavedSearch(null, " Login ", query);
  const restored = await previewBridge.savedSearchQuery(saved.id);
  expect(restored).toMatchObject({ ...query, offset: 0, limit: 500 });
  expect((await previewBridge.query(restored)).total).toBe(2);
  await expect(
    previewBridge.putSavedSearch(null, "LOGIN", query),
  ).rejects.toThrow("unique name");
  await expect(
    previewBridge.putSavedSearch(null, "Missing source", {
      ...query,
      sourceId: 999,
    }),
  ).rejects.toThrow("unavailable source");
  const list = await previewBridge.savedSearches();
  list[0].query.search = "mutated";
  expect((await previewBridge.savedSearchQuery(saved.id)).search).toBe("login");
  await previewBridge.putSavedSearch(saved.id, "Renamed", {
    ...query,
    search: "health",
  });
  expect((await previewBridge.savedSearches())[0].name).toBe("Renamed");
  await previewBridge.deleteSavedSearch(saved.id);
  expect(await previewBridge.savedSearches()).toEqual([]);
  await previewBridge.putSavedSearch(null, "Transient", query);
  await previewBridge.closeCase();
  expect(await previewBridge.savedSearches()).toEqual([]);
});
