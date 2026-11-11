import { test, expect } from "@playwright/test";

test("custom profile controls explain desktop requirements without persisting evidence in browser", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Parsers", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Local parsing profiles" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Custom profiles and preview require the desktop application.",
      { exact: false },
    ),
  ).toBeVisible();
  await expect(page.getByLabel("Profile name", { exact: true })).toHaveValue(
    "Application log",
  );
  await expect(
    page.getByLabel("Profile pattern", { exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Preview profile", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Save as new profile", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Save new revision", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("option", { name: "Custom profile — choose in Parsers" }),
  ).toHaveJSProperty("disabled", true);
  expect(
    await page.evaluate(() => Object.values(localStorage).join("")),
  ).not.toContain("Server ready");
  await page
    .getByRole("button", { name: "Select parser", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("combobox", { name: "Parser format" }),
  ).toHaveValue("apache");
  await expect(
    page.getByRole("heading", { name: "A clearer view of your logs." }),
  ).toBeVisible();
});
