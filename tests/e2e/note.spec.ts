import { expect, test } from "@playwright/test";
import { expectCleanLayout } from "./layout";

test.use({ viewport: { width: 360, height: 320 } });

for (const scheme of ["light", "dark"] as const) {
  test(`note ${scheme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("/note.html");
    await expect(page.locator(".card.open")).toBeVisible();
    await page.waitForTimeout(300);
    await expectCleanLayout(page, `note ${scheme}`);
    const area = page.locator("textarea");
    await area.fill("x ".repeat(400));
    await expect(page.getByText("400 words")).toBeVisible();
    await expectCleanLayout(page, `note ${scheme} with long text`);
    await page.screenshot({ path: `test-results/note-${scheme}.png` });
  });
}

test("escape closes the note", async ({ page }) => {
  await page.goto("/note.html");
  await expect(page.locator(".card.open")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".card.open")).toHaveCount(0);
});
