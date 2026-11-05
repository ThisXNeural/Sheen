// Shared state for the main window: library, collections and player, plus
// indexes derived from them once per change (not per render).

import * as api from "./api";

export interface Album {
  key: string;
  title: string;
  artist: string;
  year: number | null;
  tracks: api.Track[];
  durationMs: number;
}

export interface Artist {
  name: string;
  albums: Album[];
  tracks: api.Track[];
  art: string;
}

class Store {
  tracks = $state.raw<api.Track[]>([]);
  status = $state<api.LibraryStatus>({ scanning: false, count: 0, root: null });
  player = $state<api.PlayerState | null>(null);
  collections = $state.raw<api.Collections>({ liked: [], playlists: [] });

  byPath = $derived(new Map(this.tracks.map((t) => [t.path, t])));
  liked = $derived(new Set(this.collections.liked));

  albums = $derived.by(() => {
    const map = new Map<string, Album>();
    for (const t of this.tracks) {
      if (!t.album) continue;
      let a = map.get(t.art);
      if (!a) {
        a = { key: t.art, title: t.album, artist: t.albumArtist || t.artist, year: t.year, tracks: [], durationMs: 0 };
        map.set(t.art, a);
      }
      a.tracks.push(t);
      a.durationMs += t.durationMs;
      if (!a.year && t.year) a.year = t.year;
    }
    return [...map.values()].sort((x, y) => x.title.localeCompare(y.title));
  });

  albumsByKey = $derived(new Map(this.albums.map((a) => [a.key, a])));

  artists = $derived.by(() => {
    const map = new Map<string, Artist>();
    const get = (name: string, art: string) => {
      let a = map.get(name);
      if (!a) map.set(name, (a = { name, albums: [], tracks: [], art }));
      return a;
    };
    for (const t of this.tracks) get(t.albumArtist || t.artist || "Unknown artist", t.art).tracks.push(t);
    for (const album of this.albums) get(album.artist, album.key).albums.push(album);
    return [...map.values()].sort((x, y) => x.name.localeCompare(y.name));
  });

  artistsByName = $derived(new Map(this.artists.map((a) => [a.name, a])));

  /** Tracks of a playlist (or Liked), skipping files no longer in the library. */
  resolve(paths: string[]): api.Track[] {
    const out: api.Track[] = [];
    for (const p of paths) {
      const t = this.byPath.get(p);
      if (t) out.push(t);
    }
    return out;
  }

  async init() {
    const reload = async () => (this.tracks = await api.libraryTracks());
    api.onPlayer((s) => (this.player = s));
    api.onCollections((c) => (this.collections = c));
    api.onLibrary((s) => {
      const finished = this.status.scanning && !s.scanning;
      this.status = s;
      if (finished) reload();
    });
    await Promise.all([
      reload(),
      api.libraryStatus().then((s) => (this.status = s)),
      api.playerState().then((s) => s && (this.player = s)),
      api.collections().then((c) => (this.collections = c)),
    ]);
  }
}

export const store = new Store();
