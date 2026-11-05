import { expect, test } from "@playwright/test";
import { layoutIssues } from "./layout";

// The layout checker itself must catch the problems it claims to.
test("checker detects overflow, clipping, overlap and broken icons", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 300 });
  await page.setContent(`
    <div style="width:100px;overflow:hidden"><div id="spill" style="width:200px;height:10px"></div></div>
    <div id="clip" style="width:60px;overflow:hidden;white-space:nowrap">a long line of text that is cut</div>
    <div style="display:flex;width:100px"><span style="width:80px;flex:none">A</span><span style="width:80px;flex:none;margin-left:-40px">B</span></div>
    <div style="position:fixed;left:380px;top:10px;width:60px;height:10px">offscreen</div>
    <div style="height:20px;width:100px"><div style="height:40px"></div></div>
    <div style="display:grid;grid-template-columns:30px;width:30px"><button style="padding:0;display:flex">Wide label</button></div>
    <svg class="icon" data-icon="bad" width="24" height="24" viewBox="0 0 24 24"><path d="M0 0 L40 40" stroke="black"/></svg>
  `);
  const rules = (await layoutIssues(page)).map((i) => i.rule).sort();
  expect(rules).toEqual(expect.arrayContaining(["clipped-text", "escape", "icon", "overlap", "spill", "text-spill"]));
});

test("checker accepts ellipsis and scrolling content", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 300 });
  await page.setContent(`
    <div style="width:60px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis">a long line of text</div>
    <div style="width:100px;height:50px;overflow:auto"><div style="width:300px;height:300px"></div></div>
  `);
  expect(await layoutIssues(page)).toEqual([]);
});
