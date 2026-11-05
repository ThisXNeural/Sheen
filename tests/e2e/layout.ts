import { expect, type Page } from "@playwright/test";

export interface Issue {
  rule: string;
  element: string;
  detail: string;
}

/**
 * Geometric checks over every rendered element:
 *
 * - **escape**: an element sticks out of the nearest ancestor that clips it
 *   (or out of the window), i.e. it is cut off or spills.
 * - **clipped-text**: text is cut by `overflow: hidden` without an ellipsis
 *   or line clamp.
 * - **overlap**: two in-flow children of a flex/grid container intersect.
 * - **spill**: an in-flow element sticks out of its parent's box (e.g. a grid
 *   item wrapping onto an extra row below a fixed-height header).
 * - **text-spill**: text runs past the edge of its own element without
 *   being clipped (e.g. a label wider than its grid cell).
 * - **squashed**: an element with text has been squeezed to nothing.
 * - **icon**: an icon renders at zero size or draws outside its 24×24 box.
 */
export async function layoutIssues(page: Page): Promise<Issue[]> {
  return page.evaluate(() => {
    const issues: { rule: string; element: string; detail: string }[] = [];
    const TOL = 1.5;

    const describe = (el: Element) => {
      const cls = el.getAttribute("class")?.split(/\s+/).filter((c) => !c.startsWith("svelte-")).join(".");
      const text = (el.textContent ?? "").trim().replace(/\s+/g, " ").slice(0, 40);
      return `${el.tagName.toLowerCase()}${cls ? "." + cls : ""}${text ? ` "${text}"` : ""}`;
    };

    const visible = (el: Element) => {
      const cs = getComputedStyle(el);
      if (cs.display === "none" || cs.visibility === "hidden") return false;
      const r = el.getBoundingClientRect();
      return r.width > 0 || r.height > 0;
    };

    const hasOwnText = (el: Element) =>
      [...el.childNodes].some((n) => n.nodeType === Node.TEXT_NODE && n.textContent!.trim().length > 0);

    /** Nearest ancestor that clips, or null for the viewport. */
    const clipper = (el: Element): Element | null => {
      for (let p = el.parentElement; p; p = p.parentElement) {
        const cs = getComputedStyle(p);
        if (cs.overflowX !== "visible" || cs.overflowY !== "visible") return p;
        if (cs.position === "fixed") return null;
      }
      return null;
    };

    const scrolls = (el: Element) => {
      const cs = getComputedStyle(el);
      return ["auto", "scroll"].includes(cs.overflowX) || ["auto", "scroll"].includes(cs.overflowY);
    };

    for (const el of document.querySelectorAll("body *")) {
      if (!visible(el) || el.closest("svg") !== null && el.tagName.toLowerCase() !== "svg") continue;
      const cs = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      // Off-screen content (scrolled away, or skipped by content-visibility)
      // is not rendered and has no reliable geometry; it is checked when the
      // tests scroll it into view.
      if (r.bottom < 0 || r.top > innerHeight || r.right < 0 || r.left > innerWidth) continue;

      // escape
      const c = clipper(el);
      if (!(c && scrolls(c)) && cs.position !== "fixed") {
        const box = c ? c.getBoundingClientRect() : new DOMRect(0, 0, innerWidth, innerHeight);
        const out =
          r.left < box.left - TOL || r.top < box.top - TOL || r.right > box.right + TOL || r.bottom > box.bottom + TOL;
        if (out && r.width > 0 && r.height > 0) {
          issues.push({
            rule: "escape",
            element: describe(el),
            detail: `${fmt(r)} outside ${c ? describe(c) : "window"} ${fmt(box)}`,
          });
        }
      }
      if (cs.position === "fixed" && (r.right > innerWidth + TOL || r.bottom > innerHeight + TOL || r.left < -TOL || r.top < -TOL)) {
        issues.push({ rule: "escape", element: describe(el), detail: `fixed element ${fmt(r)} outside window` });
      }

      // spill: in-flow children must stay inside their parent's box.
      const parent = el.parentElement;
      if (
        parent &&
        parent !== document.body &&
        (cs.position === "static" || cs.position === "relative") &&
        // A transform draws outside the layout box on purpose (e.g. the
        // island's size setting); "escape" still checks the window edges.
        cs.transform === "none" &&
        !scrolls(parent) &&
        getComputedStyle(parent).display !== "contents" &&
        getComputedStyle(parent).display !== "inline" &&
        cs.display !== "inline"
      ) {
        const p = parent.getBoundingClientRect();
        const out = r.left < p.left - TOL || r.top < p.top - TOL || r.right > p.right + TOL || r.bottom > p.bottom + TOL;
        if (out && r.width > 0 && r.height > 0) {
          issues.push({ rule: "spill", element: describe(el), detail: `${fmt(r)} outside parent ${describe(parent)} ${fmt(p)}` });
        }
      }

      // text-spill: text wider/taller than its (non-clipping) element.
      if (hasOwnText(el) && cs.overflowX === "visible" && cs.display !== "inline") {
        for (const node of el.childNodes) {
          if (node.nodeType !== Node.TEXT_NODE || !node.textContent!.trim()) continue;
          const range = document.createRange();
          range.selectNodeContents(node);
          const t = range.getBoundingClientRect();
          if (t.width && (t.left < r.left - TOL || t.right > r.right + TOL || t.bottom > r.bottom + TOL || t.top < r.top - TOL)) {
            issues.push({ rule: "text-spill", element: describe(el), detail: `text ${fmt(t)} in box ${fmt(r)}` });
            break;
          }
        }
      }

      // clipped-text
      if (hasOwnText(el) && (cs.overflowX === "hidden" || cs.overflowX === "clip")) {
        const clamped = cs.webkitLineClamp !== "none" && cs.webkitLineClamp !== "";
        if (el.scrollWidth > el.clientWidth + 1 && cs.textOverflow !== "ellipsis" && !clamped) {
          issues.push({ rule: "clipped-text", element: describe(el), detail: `${el.scrollWidth}px of text in ${el.clientWidth}px` });
        }
      }

      // squashed
      if (hasOwnText(el) && cs.display !== "inline" && (r.width < 2 || r.height < 2)) {
        issues.push({ rule: "squashed", element: describe(el), detail: fmt(r) });
      }

      // overlap
      if (/flex|grid/.test(cs.display)) {
        const kids = [...el.children].filter((k) => {
          if (!visible(k)) return false;
          const ks = getComputedStyle(k);
          return ks.position === "static" || ks.position === "relative" || ks.position === "sticky";
        });
        for (let i = 0; i < kids.length; i++) {
          for (let j = i + 1; j < kids.length; j++) {
            const a = kids[i].getBoundingClientRect();
            const b = kids[j].getBoundingClientRect();
            const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
            const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
            if (w > TOL && h > TOL) {
              issues.push({
                rule: "overlap",
                element: describe(el),
                detail: `${describe(kids[i])} ∩ ${describe(kids[j])} (${w.toFixed(1)}×${h.toFixed(1)})`,
              });
            }
          }
        }
      }
    }

    // icons
    for (const svg of document.querySelectorAll<SVGSVGElement>("svg.icon")) {
      if (!visible(svg)) continue;
      const r = svg.getBoundingClientRect();
      if (r.width < 1 || r.height < 1) {
        issues.push({ rule: "icon", element: `svg[${svg.dataset.icon}]`, detail: `rendered at ${fmt(r)}` });
        continue;
      }
      const b = svg.getBBox();
      if (b.width === 0 && b.height === 0) {
        issues.push({ rule: "icon", element: `svg[${svg.dataset.icon}]`, detail: "draws nothing" });
      } else if (b.x < -0.5 || b.y < -0.5 || b.x + b.width > 24.5 || b.y + b.height > 24.5) {
        issues.push({ rule: "icon", element: `svg[${svg.dataset.icon}]`, detail: `draws outside 24×24: ${fmt(b)}` });
      }
    }

    return issues;

    function fmt(r: { x: number; y: number; width: number; height: number }) {
      return `[${Math.round(r.x)},${Math.round(r.y)} ${Math.round(r.width)}×${Math.round(r.height)}]`;
    }
  });
}

export async function expectCleanLayout(page: Page, label: string) {
  const issues = await layoutIssues(page);
  const report = issues.map((i) => `  ${i.rule}: ${i.element} — ${i.detail}`).join("\n");
  expect(issues, `${label}\n${report}`).toEqual([]);
}
