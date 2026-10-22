import { test, expect, type Page } from "@playwright/test";
async function sample(page: Page) {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic sample" })
    .click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
}
async function statistics(page: Page) {
  await page.getByRole("button", { name: "Statistics", exact: true }).click();
}

test("nested timeline zoom, back and reset preserve non-time filters", async ({
  page,
}) => {
  await sample(page);
  await page.getByRole("textbox", { name: "Search logs" }).fill("GET");
  await expect(page.getByText("1–10 of 10 events")).toBeVisible();
  await statistics(page);
  await page.getByLabel("Timeline interval action").selectOption("zoom");
  const bins = page.getByRole("button", { name: /Filter interval/ });
  await expect(bins.first()).toBeEnabled();
  await bins.first().click();
  await expect(
    page.getByRole("heading", { name: "Statistics", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Previous time range" }),
  ).toBeEnabled();
  await expect(page.getByRole("textbox", { name: "Search logs" })).toHaveValue(
    "GET",
  );
  await page.getByRole("button", { name: "Previous time range" }).click();
  await expect(
    page.getByRole("button", { name: "Previous time range" }),
  ).toBeDisabled();
  await expect(
    page.getByText("All timestamps · select Zoom in", { exact: false }),
  ).toBeVisible();
  await bins.first().click();
  await page.getByRole("button", { name: "Reset time range" }).click();
  await expect(
    page.getByRole("button", { name: "Reset time range" }),
  ).toBeDisabled();
  await page.getByLabel("Timeline interval action").selectOption("logs");
  await bins.first().click();
  await expect(page.locator(".table-row")).toHaveCount(1);
  await expect(page.getByRole("textbox", { name: "Search logs" })).toHaveValue(
    "GET",
  );
});

test("statistics drill-down preserves the literal search and supports chained exact filters", async ({
  page,
}) => {
  await sample(page);
  await page.getByRole("textbox", { name: "Search logs" }).fill("POST");
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await statistics(page);
  const ips = page.getByRole("region", { name: "Client IPs", exact: true });
  await expect(
    ips.getByRole("button", { name: "198.51.100.24 2" }),
  ).toBeVisible();
  await ips.getByRole("button", { name: "198.51.100.24 2" }).click();
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await expect(page.getByLabel("Investigation filters")).toContainText(
    "sourceIp = 198.51.100.24",
  );
  await statistics(page);
  await page
    .getByRole("region", { name: "Request paths", exact: true })
    .getByRole("button", { name: "/login 2" })
    .click();
  await expect(page.getByLabel("Investigation filters")).toContainText(
    "path = /login",
  );
  await expect(page.getByRole("textbox", { name: "Search logs" })).toHaveValue(
    "POST",
  );
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
});

test("save, apply, update and explicitly delete a named query without browser persistence", async ({
  page,
}) => {
  await sample(page);
  await page.getByRole("textbox", { name: "Search logs" }).fill("POST");
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await page
    .getByRole("button", { name: "Saved searches", exact: true })
    .click();
  await page
    .getByLabel("Search name", { exact: true })
    .fill("Login investigation");
  await page.getByRole("button", { name: "Save as new" }).click();
  await expect(page.getByLabel("Saved search", { exact: true })).toHaveValue(
    "1",
  );
  await page.getByRole("button", { name: "Close saved searches" }).click();
  await page.getByRole("button", { name: "Clear search" }).click();
  await expect(page.getByText("1–13 of 13 events")).toBeVisible();
  await page
    .getByRole("button", { name: "Saved searches", exact: true })
    .click();
  await page.getByRole("button", { name: "Apply search" }).click();
  await expect(page.getByRole("textbox", { name: "Search logs" })).toHaveValue(
    "POST",
  );
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  await page
    .getByRole("button", { name: "Saved searches", exact: true })
    .click();
  await page.getByLabel("Search name", { exact: true }).fill("Renamed login");
  await page.getByRole("button", { name: "Update selected" }).click();
  await expect(page.getByRole("option", { name: "Renamed login" })).toHaveCount(
    1,
  );
  await page
    .getByRole("button", { name: "Delete search", exact: true })
    .click();
  await page.getByRole("button", { name: "Keep search" }).click();
  await expect(page.getByRole("option", { name: "Renamed login" })).toHaveCount(
    1,
  );
  await page
    .getByRole("button", { name: "Delete search", exact: true })
    .click();
  await page.getByRole("button", { name: "Confirm delete search" }).click();
  await expect(page.getByText("No saved searches in this case.")).toBeVisible();
  await expect(page.getByText("1–2 of 2 events")).toBeVisible();
  expect(
    await page.evaluate(() => Object.values(localStorage).join("")),
  ).not.toContain("Login investigation");
});

test("saved-search panel closes with Escape and restores keyboard focus", async ({
  page,
}) => {
  await sample(page);
  const trigger = page.getByRole("button", {
    name: "Saved searches",
    exact: true,
  });
  await trigger.click();
  await page.getByLabel("Search name", { exact: true }).focus();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("region", { name: "Saved searches", exact: true }),
  ).toHaveCount(0);
  await expect(trigger).toBeFocused();
});
