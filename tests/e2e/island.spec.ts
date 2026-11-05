import { expect, test } from "@playwright/test";
import { expectCleanLayout } from "./layout";

// The island's native window is a fixed 700×280 transparent canvas.
test.use({ viewport: { width: 700, height: 280 } });

const modes = [
  "idle",
  "idleShelf",
  "idleHover",
  "mediaCompact",
  "mediaPeek",
  "mediaTransfer",
  "osd",
  "mic",
  "clipText",
  "clipFiles",
  "transfer",
  "transferPeek",
  "transferDone",
  "shelf",
  "shelfDrop",
  "shelfEmpty",
  "expanded",
  "expandedOsd",
];

const variants = [
  { long: false, scale: 1 },
  { long: true, scale: 1 },
  // The largest size from Settings, with side buttons, must still fit.
  { long: true, scale: 1.3 },
];

for (const scheme of ["light", "dark"] as const) {
  for (const { long, scale } of variants) {
    test.describe(`${scheme}${long ? " long text" : ""}${scale !== 1 ? ` ×${scale}` : ""}`, () => {
      test.use({ colorScheme: scheme });
      for (const mode of modes) {
        test(mode, async ({ page }) => {
          await page.goto(`/?mode=${mode}${long ? "&long" : ""}&scale=${scale}`);
          // Wait for the spring to settle on the target size.
          const capsule = page.locator(".capsule");
          await expect
            .poll(async () => {
              const a = await capsule.boundingBox();
              await page.waitForTimeout(120);
              const b = await capsule.boundingBox();
              return a && b && Math.abs(a.width - b.width) < 0.5 && Math.abs(a.height - b.height) < 0.5 && b.width > 20;
            })
            .toBe(true);
          await page.waitForTimeout(250); // content cross-fade
          await expectCleanLayout(page, `${scheme} ${mode}`);
          await page.screenshot({
            path: `test-results/island/${scheme}-${mode}${long ? "-long" : ""}-x${scale}.png`,
          });
        });
      }
    });
  }
}
