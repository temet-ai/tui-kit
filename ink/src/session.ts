import type { Connection } from "./connection.js";
import type { Size } from "./generated/Size.js";
import type { ToHost } from "./generated/ToHost.js";
import type { ToRenderer } from "./generated/ToRenderer.js";

/** tui-kit-ipc's PROTOCOL. */
export const PROTOCOL = 1;

/** How a renderer exits: the host's supervisor restarts on anything but 0 and 3. */
export const EXIT_OK = 0;
export const EXIT_ERROR = 1;
export const EXIT_NO_SOCKET = 2;
export const EXIT_REFUSED = 3;

export type SessionState<V> = {
  phase: "connecting" | "ready" | "refused" | "closed";
  /** The newest view the host sent. */
  view?: V;
  /** The cursor of that view; intents carry it. */
  cursor: number;
  /** Status lines, oldest first; never dropped. */
  notices: string[];
  /** Why the session ended, to print after the terminal is restored. */
  message?: string;
  exitCode?: number;
};

/**
 * The renderer's end of the protocol, for an app whose view type is `V` and
 * input type is `I`: Hello, then Welcome or Refuse, then each Snapshot held and
 * acknowledged, notices kept, intents stamped with the cursor they were seen at.
 */
export class Session<V, I> {
  state: SessionState<V> = { phase: "connecting", cursor: 0, notices: [] };
  private listeners = new Set<() => void>();

  constructor(
    private readonly conn: Connection,
    /** The app's view version (its Rust VIEW constant). */
    private readonly view: number,
    private size: Size,
  ) {
    conn.onMessage((m) => this.receive(m as ToRenderer<V>));
    conn.onClose((reason) => {
      if (this.state.phase === "closed" || this.state.phase === "refused") return;
      this.set({ phase: "closed", message: `host connection ${reason}`, exitCode: EXIT_ERROR });
    });
  }

  start() {
    this.send({ type: "hello", protocol: PROTOCOL, view: this.view, size: this.size });
  }

  intent(intent: I) {
    this.send({ type: "intent", cursor: this.state.cursor, intent });
  }

  resize(size: Size) {
    this.size = size;
    this.send({ type: "resize", size });
  }

  log(level: string, text: string) {
    this.send({ type: "log", level, text });
  }

  /** The renderer leaves on its own; the host takes it as a quit. */
  bye() {
    this.send({ type: "bye" });
    this.set({ phase: "closed", exitCode: EXIT_OK });
  }

  subscribe(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  private send(message: ToHost<I>) {
    this.conn.send(message);
  }

  private set(patch: Partial<SessionState<V>>) {
    this.state = { ...this.state, ...patch };
    for (const l of [...this.listeners]) l();
  }

  private receive(m: ToRenderer<V>) {
    switch (m.type) {
      case "welcome":
        this.set({ phase: "ready", cursor: m.cursor });
        break;
      case "refuse":
        this.set({ phase: "refused", message: m.message, exitCode: EXIT_REFUSED });
        break;
      case "snapshot":
        this.set({ view: m.view, cursor: m.cursor });
        this.send({ type: "ack", cursor: m.cursor });
        break;
      case "notice":
        this.set({ notices: [...this.state.notices, m.text] });
        break;
      case "shutdown":
        this.set({ phase: "closed", exitCode: EXIT_OK });
        break;
    }
  }
}
