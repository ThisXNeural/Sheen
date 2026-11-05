export type Route =
  | { view: "songs" }
  | { view: "albums" }
  | { view: "album"; key: string }
  | { view: "artists" }
  | { view: "artist"; name: string }
  | { view: "liked" }
  | { view: "playlist"; id: number }
  | { view: "settings" };

class Router {
  route = $state<Route>(initial());
  private history: Route[] = [];

  go(route: Route) {
    if (JSON.stringify(route) === JSON.stringify(this.route)) return;
    this.history.push(this.route);
    if (this.history.length > 50) this.history.shift();
    this.route = route;
  }

  /** Top-level navigation from the sidebar resets the back stack. */
  root(route: Route) {
    this.history = [];
    this.route = route;
  }

  back() {
    this.route = this.history.pop() ?? { view: "songs" };
  }

  get canGoBack() {
    return this.history.length > 0;
  }
}

/** `?view=albums` etc. lets the tests open any page directly. */
function initial(): Route {
  const p = new URLSearchParams(location.search);
  switch (p.get("view")) {
    case "albums":
      return { view: "albums" };
    case "album":
      return { view: "album", key: p.get("key") ?? "" };
    case "artists":
      return { view: "artists" };
    case "artist":
      return { view: "artist", name: p.get("name") ?? "" };
    case "liked":
      return { view: "liked" };
    case "playlist":
      return { view: "playlist", id: Number(p.get("id") ?? 0) };
    case "settings":
      return { view: "settings" };
    default:
      return { view: "songs" };
  }
}

export const router = new Router();
