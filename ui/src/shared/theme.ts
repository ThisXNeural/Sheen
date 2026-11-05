// Light/dark handling shared by both windows.
//
// "system" follows the Windows app theme: WebView2 reports it through
// `prefers-color-scheme` and fires `change` when the user switches. An
// explicit choice simply overrides it. The resolved theme is written to
// <html data-theme>, which the CSS tokens key off.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { inTauri } from "./env";

export type ThemePref = "system" | "light" | "dark";

const media = window.matchMedia("(prefers-color-scheme: dark)");
let pref: ThemePref = "system";

function apply() {
  const dark = pref === "dark" || (pref === "system" && media.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

media.addEventListener("change", apply);

/** Apply the saved preference now and track changes from any window. */
export async function initTheme(onChange?: (p: ThemePref) => void) {
  const set = (p: ThemePref) => {
    pref = p;
    apply();
    onChange?.(p);
  };
  apply();
  if (!inTauri) {
    const forced = new URLSearchParams(location.search).get("theme") as ThemePref | null;
    if (forced) set(forced);
    return;
  }
  await listen<{ theme: ThemePref }>("sheen://settings", (e) => set(e.payload.theme));
  const settings = await invoke<{ theme: ThemePref }>("get_settings");
  set(settings.theme);
}

export function setTheme(p: ThemePref) {
  return invoke("set_theme", { theme: p });
}
