// Track actions shared by every list, the player bar and the context menu.

import * as api from "./api";
import { menu, prompt, toast, type MenuItem } from "./menu.svelte";
import { router } from "./router.svelte";
import { store } from "./store.svelte";

const ids = (tracks: api.Track[]) => tracks.map((t) => t.id);
const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? "" : "s"}`;

export function playAll(tracks: api.Track[], start = 0, shuffle = false) {
  if (!tracks.length) return;
  if (shuffle) {
    api.command({ type: "setShuffle", on: true });
    start = Math.floor(Math.random() * tracks.length);
  }
  api.playTracks(ids(tracks), start);
}

export async function newPlaylist(tracks: api.Track[] = []) {
  const name = await prompt.ask("New playlist", "", "Create");
  if (name == null) return;
  const id = await api.playlistCreate(name.trim() || "New playlist", ids(tracks));
  if (tracks.length) toast.show(`Added ${plural(tracks.length, "song")} to ${name.trim() || "New playlist"}`);
  router.go({ view: "playlist", id });
}

export async function addToPlaylist(id: number, tracks: api.Track[]) {
  const added = await api.playlistAdd(id, ids(tracks));
  const name = store.collections.playlists.find((p) => p.id === id)?.name ?? "playlist";
  toast.show(added ? `Added ${plural(added, "song")} to ${name}` : `Already in ${name}`);
}

export function trackMenu(tracks: api.Track[], opts: { playlistId?: number; list?: api.Track[]; index?: number } = {}): MenuItem[] {
  const single = tracks.length === 1 ? tracks[0] : null;
  const liked = single ? store.liked.has(single.path) : false;
  const items: MenuItem[] = [
    {
      label: "Play",
      icon: "play",
      action: () =>
        single && opts.list && opts.index != null ? playAll(opts.list, opts.index) : playAll(tracks),
    },
    {
      label: "Play next",
      icon: "listPlus",
      action: () => {
        api.enqueue(ids(tracks), true);
        toast.show(single ? `“${single.title}” plays next` : `${plural(tracks.length, "song")} play next`);
      },
    },
    {
      label: "Add to queue",
      icon: "listEnd",
      action: () => {
        api.enqueue(ids(tracks), false);
        toast.show(`Added ${plural(tracks.length, "song")} to the queue`);
      },
    },
    {
      label: "Add to playlist",
      icon: "playlists",
      submenu: [
        { label: "New playlist…", icon: "plus", action: () => newPlaylist(tracks) },
        ...(store.collections.playlists.length ? [{ label: "", separator: true }] : []),
        ...store.collections.playlists.map((p) => ({ label: p.name, action: () => addToPlaylist(p.id, tracks) })),
      ],
    },
    { label: "", separator: true },
  ];
  if (single) {
    items.push({
      label: liked ? "Remove from Liked Songs" : "Add to Liked Songs",
      icon: "heart",
      action: () => api.toggleLike(single.path),
    });
    if (single.album) items.push({ label: "Go to album", icon: "albums", action: () => router.go({ view: "album", key: single.art }) });
    items.push({
      label: "Go to artist",
      icon: "artists",
      action: () => router.go({ view: "artist", name: single.albumArtist || single.artist || "Unknown artist" }),
    });
    items.push({ label: "Show in Explorer", icon: "folderOpen", action: () => api.reveal(single.path) });
  }
  if (opts.playlistId != null) {
    const id = opts.playlistId;
    items.push({ label: "", separator: true });
    items.push({
      label: "Remove from this playlist",
      icon: "trash",
      danger: true,
      action: () => {
        const playlist = store.collections.playlists.find((p) => p.id === id);
        if (!playlist) return;
        const paths = new Set(tracks.map((t) => t.path));
        const positions = playlist.tracks.flatMap((p, i) => (paths.has(p) ? [i] : []));
        api.playlistRemove(id, positions);
      },
    });
  }
  return items;
}

export function openTrackMenu(e: MouseEvent, tracks: api.Track[], opts: Parameters<typeof trackMenu>[1] = {}) {
  e.preventDefault();
  menu.show(e.clientX, e.clientY, trackMenu(tracks, opts));
}
