import { expect, test } from "@playwright/test";
import { expectCleanLayout } from "./layout";

// The island's native window is a fixed 640×280 transparent canvas.
test.use({ viewport: { width: 640, height: 280 } });

const modes = [
  "idle",
  "idleShelf",
  "mediaCompact",
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

for (const scheme of ["light", "dark"] as const) {
  for (const long of [false, true]) {
    test.describe(`${scheme}${long ? " long text" : ""}`, () => {
      test.use({ colorScheme: scheme });
      for (const mode of modes) {
        test(mode, async ({ page }) => {
          await page.goto(`/?mode=${mode}${long ? "&long" : ""}`);
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
          await page.screenshot({ path: `test-results/island/${scheme}-${mode}${long ? "-long" : ""}.png` });
        });
      }
    });
  }
}
