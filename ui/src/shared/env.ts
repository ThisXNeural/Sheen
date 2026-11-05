/** False when the UI is opened in a plain browser (`pnpm dev`). */
export const inTauri = "__TAURI_INTERNALS__" in window;
