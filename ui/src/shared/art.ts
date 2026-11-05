import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "./env";

export type ArtKind = "thumb" | "cover" | "shelf";

/** URL of an image served by the `sheen-art` protocol (null in the browser demo). */
export function artUrl(kind: ArtKind, key: string | number | null | undefined): string | null {
  if (!inTauri || key == null || key === "") return null;
  return convertFileSrc(`${kind}/${key}`, "sheen-art");
}

/** A stable hue for placeholder art, so every album gets its own colour. */
export function hueFor(key: string): number {
  let h = 0;
  for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return h % 360;
}
