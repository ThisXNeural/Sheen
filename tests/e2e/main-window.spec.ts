import { expect, test } from "@playwright/test";
import { expectCleanLayout } from "./layout";

const sizes = [
  { width: 720, height: 480 }, // the window's minimum size
  { width: 1000, height: 660 }, // default
  { width: 1600, height: 1000 },
];

const routes = [
  "view=songs",
  "view=albums",
  "view=album&key=album-0",
  "view=album&key=album-4", // very long album name
  "view=artists",
  "view=artist&name=" + encodeURIComponent("An Artist With A Remarkably Long Name That Never Seems To End"),
  "view=artist&name=" + encodeURIComponent("گروه نمونه"),
  "view=liked",
  "view=playlist&id=1",
  "view=playlist&id=2",
  "view=playlist&id=3",
  "view=settings",
  "view=songs&queue",
  "view=albums&queue",
  "view=songs&library=empty",
];

for (const scheme of ["light", "dark"] as const) {
  for (const size of sizes) {
    test.describe(`${scheme} ${size.width}×${size.height}`, () => {
      test.use({ viewport: size, colorScheme: scheme });

      for (const route of routes) {
        test(route, async ({ page }) => {
          const errors: string[] = [];
          page.on("pageerror", (e) => errors.push(e.message));
          await page.goto(`/main.html?${route}`);
          await page.waitForLoadState("networkidle");
          await page.waitForTimeout(150);
          await expectCleanLayout(page, `${scheme} ${size.width}×${size.height} ${route}`);
          expect(errors).toEqual([]);
        });
      }
    });
  }
}

test.describe("interactions", () => {
  test.use({ viewport: { width: 720, height: 480 } });

  test("context menu and submenu stay on screen", async ({ page }) => {
    await page.goto("/main.html?view=songs");
    // Right-click near the bottom-right corner of the list, where a naive
    // menu would run off the window.
    const box = (await page.locator(".list").boundingBox())!;
    await page.mouse.click(box.x + box.width - 12, box.y + box.height - 12, { button: "right" });
    await expect(page.getByRole("menu")).toBeVisible();
    await page.getByRole("menuitem", { name: "Add to playlist" }).hover();
    await expect(page.getByRole("menu")).toHaveCount(2);
    await expectCleanLayout(page, "context menu open");
    // No dangling separator at the end of a menu.
    const lastChild = page.getByRole("menu").first().locator(":scope > *").last();
    await expect(lastChild).toHaveAttribute("role", "menuitem");
  });

  test("new playlist dialog", async ({ page }) => {
    await page.goto("/main.html?view=songs");
    await page.getByRole("button", { name: "New playlist" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.type("A brand new playlist with a long name");
    await expectCleanLayout(page, "prompt dialog");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "A brand new playlist with a long name" })).toBeVisible();
    await expectCleanLayout(page, "new playlist page");
  });

  test("multi-select, play next and toast", async ({ page }) => {
    await page.goto("/main.html?view=songs");
    const rows = page.locator(".list .row");
    await rows.nth(0).click();
    await rows.nth(3).click({ modifiers: ["Shift"] });
    await expect(page.locator(".list .row.selected")).toHaveCount(4);
    await rows.nth(2).click({ button: "right" });
    await page.getByRole("menuitem", { name: "Play next" }).click();
    await expect(page.getByRole("status")).toContainText("4 songs play next");
    await expectCleanLayout(page, "toast visible");
  });

  test("search filters and keyboard shortcuts work", async ({ page }) => {
    await page.goto("/main.html?view=settings");
    await page.keyboard.press("Control+f"); // jumps to Songs and focuses search
    await expect(page.locator("[data-search]")).toBeFocused();
    await page.keyboard.type("glass");
    await expect(page.locator(".list .row").first()).toContainText("Glass");
    await page.locator(".list").click();
    await page.keyboard.press("Escape");
    const playing = page.getByRole("button", { name: /^(Play|Pause)$/ }).last();
    const before = await playing.getAttribute("aria-label");
    await page.keyboard.press(" ");
    await expect(playing).not.toHaveAttribute("aria-label", before!);
  });

  test("shortcut recorder rejects plain letters", async ({ page }) => {
    await page.goto("/main.html?view=settings");
    await page.getByRole("button", { name: /Scroll Lock/ }).click();
    await page.keyboard.press("k");
    await expect(page.getByText(/needs Ctrl, Alt, Shift or Win/)).toBeVisible();
    await page.getByRole("button", { name: /Scroll Lock/ }).click();
    await page.keyboard.press("Control+Alt+n");
    await expect(page.getByRole("button", { name: "Ctrl + Alt + N" })).toBeVisible();
    await expectCleanLayout(page, "settings after shortcut change");
  });
});
