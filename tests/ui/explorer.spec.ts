import { expect, test } from "@playwright/test";

test("sample exploration searches raw content and shows the original event safely", async ({
  page,
}) => {
  const outgoing: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith("http://127.0.0.1:1420/"))
      outgoing.push(request.url());
  });
  await page.goto("/");
  await expect(
    page.getByText("Interface preview · synthetic data only.", {
      exact: false,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.getByRole("row").filter({ hasText: "<img src=" }).click();
  await expect(page.locator(".detail-panel .raw-event").first()).toContainText(
    "<img src=",
  );
  await expect(page.locator(".detail-panel img")).toHaveCount(0);
  await page.getByRole("button", { name: "Close event details" }).click();
  await page.getByRole("textbox", { name: "Search logs" }).fill("POST /LOGIN");
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await page
    .getByRole("row")
    .filter({ hasText: "198.51.100.24" })
    .first()
    .click();
  await expect(
    page.getByRole("complementary", { name: "Event details" }),
  ).toBeVisible();
  await expect(page.locator(".raw-event").first()).toContainText(
    '"POST /login HTTP/1.1" 401',
  );
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("complementary", { name: "Event details" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Clear search" }).click();
  await page.getByRole("button", { name: "Warnings", exact: false }).click();
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.getByRole("row").filter({ hasText: "[invalid date]" }).click();
  await expect(
    page.getByText("Invalid Apache timestamp or timezone."),
  ).toBeVisible();
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(outgoing).toEqual([]);
});

test("cancellation returns to the empty session", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await page.getByRole("button", { name: "Cancel import" }).click();
  await expect(
    page.getByText("Import cancelled. No events from this import were kept."),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "A clearer view of your logs." }),
  ).toBeVisible();
});

test("search is literal, empty results are explained, and browser imports are explicit", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Import log file", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Open the desktop application",
  );
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await page.getByRole("textbox", { name: "Search logs" }).fill("' OR 1=1 --");
  await expect(page.getByText("No matching events")).toBeVisible();
  await expect(page.getByText("0–0 of 0 events")).toBeVisible();
  await page.getByRole("button", { name: "Clear search" }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("textbox", { name: "Search logs" }),
  ).toBeFocused();
  await page.screenshot({
    path: "/tmp/dfir-mvp0-explorer.png",
    fullPage: true,
  });
});

async function loadSample(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
}

test("logs occupy the main viewport and secondary controls open on demand", async ({
  page,
}) => {
  await loadSample(page);
  await expect(page.locator(".app-layout")).toHaveClass(/sidebar-collapsed/);
  await expect(page.getByLabel("Source parsing statistics")).toHaveCount(0);
  await expect(page.getByLabel("Event timeline")).toHaveCount(0);
  await expect(page.getByLabel("Timestamp presence")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Additional filters" }),
  ).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("columnheader")).toHaveCount(4);
  for (const viewport of [
    { width: 1440, height: 940 },
    { width: 1280, height: 720 },
    { width: 1000, height: 650 },
  ]) {
    await page.setViewportSize(viewport);
    const table = await page.locator(".table-shell").boundingBox();
    const firstRow = await page.locator(".table-row").first().boundingBox();
    expect(firstRow!.y).toBeLessThanOrEqual(220);
    expect(table!.height).toBeGreaterThan(viewport.height * 0.6);
    expect(
      await page.evaluate(() => ({
        width: document.documentElement.scrollWidth,
        height: document.documentElement.scrollHeight,
      })),
    ).toEqual(viewport);
    await expect(
      page.getByRole("columnheader", { name: "Raw log preview" }),
    ).toBeInViewport();
  }
  await page.setViewportSize({ width: 1440, height: 940 });
  await page.screenshot({ path: "/tmp/dfir-minimal-light.png" });
  await page.getByRole("row").filter({ hasText: "192.0.2.14" }).click();
  await page.getByRole("tab", { name: "Parse Diagnostics" }).click();
  await page.screenshot({ path: "/tmp/dfir-minimal-inspector.png" });
  await page.keyboard.press("Escape");
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("dark");
  await page.screenshot({ path: "/tmp/dfir-minimal-dark.png" });
  await page.getByRole("button", { name: "Case actions" }).click();
  await expect(
    page.getByRole("button", { name: "Save case as…" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Save case as…" })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole("button", { name: "Case actions" }),
  ).toBeFocused();
});

test("legacy default columns become compact while custom layouts survive", async ({
  page,
}) => {
  const previous = [
    "sourceId",
    "lineNumber",
    "timestamp",
    "sourceIp",
    "method",
    "path",
    "status",
    "parseStatus",
    "preview",
  ];
  await page.goto("/");
  await page.evaluate(
    (columns) => localStorage.setItem("dfir.columns", JSON.stringify(columns)),
    previous,
  );
  await loadSample(page);
  await expect(page.getByRole("columnheader")).toHaveCount(4);
  const custom = ["lineNumber", "sourceIp", "preview"];
  await page.evaluate((columns) => {
    localStorage.removeItem("dfir.columns.v2");
    localStorage.setItem("dfir.columns", JSON.stringify(columns));
  }, custom);
  await loadSample(page);
  await expect(page.getByRole("columnheader")).toHaveCount(3);
  await expect(
    page.getByRole("columnheader", { name: "Client IP" }),
  ).toBeVisible();
  await expect(
    page.getByRole("columnheader", { name: "Timestamp / source offset" }),
  ).toHaveCount(0);
  await page.reload();
  await loadSample(page);
  await expect(page.getByRole("columnheader")).toHaveCount(3);
});

test("parsing states retain raw evidence, independent warnings and real aggregates", async ({
  page,
}) => {
  await loadSample(page);
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  const summary = page.getByLabel("Source parsing statistics");
  await expect(summary).toContainText("11");
  await expect(page.getByLabel("Filtered HTTP statistics")).toContainText(
    "12 requests",
  );
  await expect(page.getByLabel("Event timeline")).toContainText(
    "2 without valid timestamp",
  );
  await page.getByRole("button", { name: "Back to logs" }).click();
  const filters = page.getByLabel("Parsing status filters");
  await filters.getByRole("button", { name: "Partial", exact: true }).click();
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.getByRole("row").filter({ hasText: "192.0.2.14" }).click();
  const inspector = page.getByRole("complementary", { name: "Event details" });
  await expect(inspector).toContainText("Partial");
  await expect(inspector).toContainText(
    "Invalid Apache timestamp or timezone.",
  );
  await expect(inspector).toContainText("192.0.2.14");
  await inspector.getByRole("tab", { name: "Raw Log", exact: true }).click();
  await expect(inspector.locator(".raw-event")).toContainText("[invalid date]");
  await inspector.getByRole("tab", { name: "Parse Diagnostics" }).click();
  await expect(inspector).toContainText("timestamp or timezone");
  await page.keyboard.press("Escape");
  await filters.getByRole("button", { name: "Unparsed", exact: true }).click();
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await expect(page.getByLabel("Event timeline")).toContainText(
    "1 without valid timestamp",
  );
  await expect(page.getByLabel("Filtered HTTP statistics")).toContainText(
    "0 requests",
  );
  await page.getByRole("button", { name: "Back to logs" }).click();
  await page.getByRole("row").filter({ hasText: "<img src=" }).click();
  await expect(inspector.locator(".raw-event")).toContainText("<img src=");
  await expect(inspector.locator("img")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Warnings", exact: false }).click();
  await expect(page.getByText("0–0 of 0 events")).toBeVisible();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
});

test("field searches, exact filters, sorting, timeline and configurable columns work together", async ({
  page,
}) => {
  await loadSample(page);
  await page.getByRole("button", { name: "Additional filters" }).click();
  await page
    .getByRole("combobox", { name: "Search field", exact: true })
    .selectOption("timestamp");
  await page
    .getByRole("textbox", { name: "Search logs" })
    .fill("2026-10-13T09:14:00");
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.getByRole("button", { name: "Columns", exact: true }).click();
  await page
    .getByRole("checkbox", { name: "HTTP status", exact: true })
    .check();
  await page.getByRole("button", { name: "Columns", exact: true }).click();
  await page
    .getByRole("columnheader", { name: "HTTP status" })
    .getByRole("button")
    .click();
  await page
    .getByRole("columnheader", { name: "HTTP status" })
    .getByRole("button")
    .click();
  await expect(
    page.getByRole("columnheader", { name: "HTTP status" }),
  ).toHaveAttribute("aria-sort", "descending");
  await expect(page.locator(".table-row").first()).toContainText("502");
  await page.getByRole("button", { name: "Columns", exact: true }).click();
  await page.getByRole("checkbox", { name: "User-Agent", exact: true }).check();
  await page.getByRole("button", { name: "Columns", exact: true }).click();
  await expect(
    page.getByRole("columnheader", { name: "User-Agent", exact: true }),
  ).toHaveCount(1);
  await page
    .getByRole("row")
    .filter({ hasText: "198.51.100.24" })
    .first()
    .click();
  await page
    .getByRole("button", { name: "Filter by Client IP / host", exact: true })
    .click();
  await expect(page.getByText("1–3 of 3 events")).toBeVisible();
  await expect(page.locator(".filter-chips")).toContainText("198.51.100.24");
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await page
    .getByRole("button", { name: "Filter interval", exact: false })
    .first()
    .click();
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await expect(page.locator(".filter-chips")).toContainText("UTC");
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await page.getByRole("button", { name: "Additional filters" }).click();
  await page.getByRole("button", { name: "Time range", exact: true }).click();
  await page.getByLabel("Time from UTC").fill("2026-10-13T09:14");
  await page.getByLabel("Time to UTC").fill("2026-10-13T09:14:01");
  await page.getByRole("button", { name: "Apply time range" }).click();
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.reload();
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await expect(
    page.getByRole("columnheader", { name: "User-Agent", exact: true }),
  ).toHaveCount(1);
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
});

test("themes, navigation, evidence, inspector closure and desktop dimensions", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("system");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("dark");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("light");
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.getByRole("button", { name: "Evidence", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Active evidence" }),
  ).toBeVisible();
  await expect(page.locator(".hash-value")).toHaveText(/[a-f0-9]{64}/);
  await page.getByRole("button", { name: "Parsers", exact: true }).click();
  await expect(page.locator(".parser-card")).toHaveCount(9);
  await page
    .getByRole("button", { name: "Select parser", exact: true })
    .first()
    .click();
  await expect(page.getByLabel("Parser format")).toHaveValue("apache");
  await page
    .getByRole("button", { name: "Expand sidebar", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Collapse sidebar", exact: true })
    .click();
  await expect(page.locator(".app-layout")).toHaveClass(/sidebar-collapsed/);
  await page.setViewportSize({ width: 1440, height: 940 });
  await page.locator(".global-search input").blur();
  await page.screenshot({
    path: "/tmp/dfir-refonte-light.png",
    fullPage: true,
  });
  await page.getByRole("row").filter({ hasText: "192.0.2.14" }).click();
  await page.getByRole("tab", { name: "Parse Diagnostics" }).click();
  await page.screenshot({
    path: "/tmp/dfir-refonte-inspector.png",
    fullPage: true,
  });
  await page
    .getByRole("combobox", { name: "Theme preference" })
    .selectOption("dark");
  await page.screenshot({ path: "/tmp/dfir-refonte-dark.png", fullPage: true });
  await page.setViewportSize({ width: 1000, height: 650 });
  const table = await page.locator(".table-shell").boundingBox();
  expect(table!.height).toBeGreaterThan(100);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Close event details" }).click();
  await expect(
    page.getByRole("complementary", { name: "Event details" }),
  ).toHaveCount(0);
});

test("keyboard navigation and clipboard preserve complete selected raw content", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await loadSample(page);
  await page.locator('.table-row[data-index="0"]').focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.locator('.table-row[data-index="1"]')).toBeFocused();
  await page.keyboard.press("Enter");
  const inspector = page.getByRole("complementary", { name: "Event details" });
  await expect(
    inspector.getByRole("heading", { name: "Line 2", exact: true }),
  ).toBeVisible();
  const raw = await inspector.locator(".raw-event").textContent();
  await inspector
    .getByRole("button", { name: "Copy raw log", exact: true })
    .click();
  await expect(inspector.getByText("Copied to clipboard.")).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(raw);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await page.getByLabel("Timeline distribution").selectOption("http");
  await expect(page.locator(".chart-legend")).toContainText("2xx");
});

test("case controls, source provenance and timestamp filters are usable", async ({
  page,
}) => {
  await loadSample(page);
  await page.getByLabel("Parser format").selectOption("nginx");
  await page.getByLabel("Parser format").selectOption("json_lines");
  await page.getByLabel("Parser format").selectOption("auto");
  await page.getByRole("button", { name: "Case actions" }).click();
  await expect(page.getByLabel("Case management")).toContainText(
    "Temporary case",
  );
  await expect(page.getByLabel("Case management")).toContainText("1 sources");
  await page.getByRole("button", { name: "Save case as…" }).click();
  await expect(page.getByRole("alert")).toContainText("desktop application");
  await page.getByRole("combobox", { name: "Source filter" }).selectOption("1");
  await page.getByRole("button", { name: "Additional filters" }).click();
  await page
    .getByRole("combobox", { name: "Timestamp presence" })
    .selectOption("false");
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await page
    .getByRole("combobox", { name: "Reader filter" })
    .selectOption("apache");
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await page.getByRole("button", { name: "Additional filters" }).click();
  await expect(page.getByLabel("Timestamp presence")).toHaveCount(0);
  await expect(page.locator(".filter-chips")).toContainText("Reader: apache");
  await expect(page.locator(".filter-chips")).toContainText(
    "Timestamp: missing / invalid",
  );
  await page.locator(".table-row").click();
  await expect(page.locator(".event-provenance")).toContainText("Source #1");
  await expect(page.locator(".event-provenance")).toContainText("SHA-256:");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page.getByRole("button", { name: "Case actions" }).click();
  await page.getByRole("button", { name: "New case", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "A clearer view of your logs." }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Case actions" }).click();
  await expect(page.getByLabel("Case management")).toContainText("0 sources");
});

test("Statistics keeps the timeline readable at desktop sizes and in both themes", async ({
  page,
}) => {
  await loadSample(page);
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Statistics", exact: true }),
  ).toHaveCount(1);
  const timeline = page.getByLabel("Event timeline");
  await expect(timeline).toContainText("11 timestamped events");
  await expect(page.getByLabel("Distribution legend")).toContainText(
    "Parsed 11",
  );
  for (const theme of ["light", "dark"]) {
    await page
      .getByRole("combobox", { name: "Theme preference" })
      .selectOption(theme);
    for (const viewport of [
      { width: 1440, height: 940 },
      { width: 1280, height: 720 },
      { width: 1000, height: 650 },
    ]) {
      await page.setViewportSize(viewport);
      const plot = await page.locator(".histogram").boundingBox();
      expect(plot!.height).toBeGreaterThanOrEqual(240);
      expect(plot!.width).toBeGreaterThan(viewport.width * 0.65);
      expect(
        await page
          .getByLabel("Time scale in UTC")
          .evaluate((el) => parseFloat(getComputedStyle(el).fontSize)),
      ).toBeGreaterThanOrEqual(11);
      for (const tick of await page.locator(".timeline-axis > span").all()) {
        expect((await tick.boundingBox())!.height).toBeLessThanOrEqual(40);
      }
      await page.getByLabel("Distribution legend").scrollIntoViewIfNeeded();
      await expect(page.getByLabel("Distribution legend")).toBeInViewport();
      await page
        .getByLabel("Source parsing statistics")
        .scrollIntoViewIfNeeded();
      const total = page.locator(".summary-total strong");
      await expect(total).toBeInViewport();
      expect(
        await total.evaluate((el) => parseFloat(getComputedStyle(el).fontSize)),
      ).toBeGreaterThanOrEqual(26);
      await page
        .getByLabel("Filtered HTTP statistics")
        .scrollIntoViewIfNeeded();
      await expect(
        page.getByLabel("Filtered HTTP statistics"),
      ).toBeInViewport();
      expect(
        await page.evaluate(() => ({
          width: document.documentElement.scrollWidth,
          height: document.documentElement.scrollHeight,
        })),
      ).toEqual(viewport);
      expect(
        await page
          .locator(".statistics-view")
          .evaluate((el) => el.scrollWidth <= el.clientWidth),
      ).toBe(true);
    }
  }
});

test("timeline details support hover, keyboard selection and filtered empty states", async ({
  page,
}) => {
  await loadSample(page);
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  const first = page
    .getByRole("button", { name: "Filter interval", exact: false })
    .first();
  const details = page.getByLabel("Interval details");
  await first.hover();
  await expect(details).toContainText("Parsed: 1");
  await expect(details).toContainText("2026-10-13 07:14:22 UTC");
  await page.getByRole("heading", { name: "Statistics", exact: true }).hover();
  await expect(details).toContainText("Hover or focus");
  await page.getByLabel("Timeline distribution").selectOption("http");
  await expect(page.getByLabel("Distribution legend")).toContainText("2xx 6");
  await first.focus();
  await expect(details).toContainText("2xx: 1");
  await page.keyboard.press("Enter");
  await expect(page.getByText("1–1 of 1 events")).toBeVisible();
  await expect(page.locator(".filter-chips")).toContainText("UTC");
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await expect(page.getByLabel("Event timeline")).toContainText(
    "Matching events · current filters",
  );
  await expect(page.getByLabel("Event timeline")).toContainText(
    "1 timestamped events",
  );
  await expect(page.getByLabel("Source parsing statistics")).toContainText(
    "13",
  );
  await expect(page.getByLabel("Filtered HTTP statistics")).toContainText(
    "1 requests",
  );
  await page.getByRole("button", { name: "Back to logs" }).click();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Search logs" })
    .fill("no such evidence");
  await expect(page.getByText("0–0 of 0 events")).toBeVisible();
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await expect(page.getByLabel("Event timeline")).toContainText(
    "No events match the current filters",
  );
  await expect(page.locator(".histogram")).toHaveCount(0);
  await page.getByRole("button", { name: "Back to logs" }).click();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await page
    .getByLabel("Parsing status filters")
    .getByRole("button", { name: "Unparsed", exact: true })
    .click();
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
  await expect(page.getByLabel("Event timeline")).toContainText(
    "1 without valid timestamp",
  );
  await expect(page.getByLabel("Event timeline")).toContainText(
    "No valid timestamps in these results",
  );
});

test("security readers expose explicit scope and combined attribute filters", async ({
  page,
}) => {
  await page.goto("/");
  const picker = page.getByRole("combobox", { name: "Parser format" });
  for (const reader of ["syslog", "iptables", "haproxy", "modsecurity"]) {
    await picker.selectOption(reader);
    await expect(picker).toHaveValue(reader);
  }
  await page.getByRole("button", { name: "Parsers", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "ModSecurity audit" }),
  ).toBeVisible();
  await expect(
    page.getByText("Native serial audit · multiline", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Explore", exact: true }).click();
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await page.getByRole("button", { name: "Additional filters" }).click();
  await page
    .getByRole("combobox", { name: "Reader filter" })
    .selectOption("iptables");
  await expect(page.getByText("No matching events")).toBeVisible();
  await page.getByRole("combobox", { name: "Reader filter" }).selectOption("");
  await page
    .getByRole("textbox", { name: "Attribute or JSON field name" })
    .fill("firewall.DPT");
  await page
    .getByRole("textbox", { name: "Attribute or JSON field value" })
    .fill("443");
  await page.getByRole("button", { name: "Apply attribute filter" }).click();
  await expect(page.getByText("No matching events")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "firewall.DPT = 443 ×" }),
  ).toBeVisible();
});
