declare module "@tauri-apps/plugin-global-shortcut" {
  export type ShortcutEventState = "Pressed" | "Released";

  export interface ShortcutEvent {
    shortcut: string;
    id: number;
    state: ShortcutEventState;
  }

  export type ShortcutHandler = (event: ShortcutEvent) => void | Promise<void>;

  export function register(
    shortcut: string | string[],
    handler: ShortcutHandler
  ): Promise<void>;
  export function unregister(shortcut: string | string[]): Promise<void>;
  export function unregisterAll(): Promise<void>;
  export function isRegistered(shortcut: string): Promise<boolean>;
}
