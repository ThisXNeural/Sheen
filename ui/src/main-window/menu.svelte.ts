import type { IconName } from "../shared/icons";

export interface MenuItem {
  label: string;
  icon?: IconName;
  action?: () => void;
  submenu?: MenuItem[];
  danger?: boolean;
  separator?: boolean;
  disabled?: boolean;
}

class Menu {
  open = $state(false);
  x = $state(0);
  y = $state(0);
  items = $state.raw<MenuItem[]>([]);

  show(x: number, y: number, items: MenuItem[]) {
    this.x = x;
    this.y = y;
    this.items = items;
    this.open = true;
  }

  close() {
    this.open = false;
  }
}

export const menu = new Menu();

/** Prompt for a line of text (new playlist name, rename). */
class Prompt {
  open = $state(false);
  title = $state("");
  value = $state("");
  confirm = $state("OK");
  private resolve: ((v: string | null) => void) | null = null;

  ask(title: string, value = "", confirm = "OK"): Promise<string | null> {
    this.title = title;
    this.value = value;
    this.confirm = confirm;
    this.open = true;
    return new Promise((r) => (this.resolve = r));
  }

  done(v: string | null) {
    this.open = false;
    this.resolve?.(v);
    this.resolve = null;
  }
}

export const prompt = new Prompt();

/** Brief confirmation at the bottom of the window. */
class Toast {
  text = $state<string | null>(null);
  private timer: ReturnType<typeof setTimeout> | undefined;

  show(text: string) {
    this.text = text;
    clearTimeout(this.timer);
    this.timer = setTimeout(() => (this.text = null), 2200);
  }
}

export const toast = new Toast();
