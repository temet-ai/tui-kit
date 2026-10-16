import net from "node:net";

/** One JSON message per newline-terminated UTF-8 line (tui-kit-ipc's framing). */
export interface Connection {
  send(message: unknown): void;
  onMessage(handler: (message: unknown) => void): void;
  onClose(handler: (reason: string) => void): void;
  close(): void;
}

/** tui-kit-ipc's MAX_LINE. */
export const MAX_LINE_BYTES = 16 * 1024 * 1024;

/** Connects to the host's socket (its path is in the variable the host names). */
export function connect(path: string): Promise<Connection> {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(path);
    const messageHandlers: Array<(m: unknown) => void> = [];
    const closeHandlers: Array<(r: string) => void> = [];
    let pending: Buffer[] = [];
    let pendingBytes = 0;
    let closed = false;

    const finish = (reason: string) => {
      if (closed) return;
      closed = true;
      for (const h of closeHandlers) h(reason);
    };

    const deliver = (line: Buffer) => {
      if (line.length === 0) return;
      let parsed: unknown;
      try {
        parsed = JSON.parse(line.toString("utf8"));
      } catch {
        socket.destroy();
        finish("invalid json");
        return;
      }
      for (const h of messageHandlers) h(parsed);
    };

    socket.on("data", (chunk: Buffer) => {
      let start = 0;
      for (;;) {
        const nl = chunk.indexOf(0x0a, start);
        if (nl === -1) break;
        const part = chunk.subarray(start, nl);
        if (pendingBytes + part.length > MAX_LINE_BYTES) {
          socket.destroy();
          finish("line too long");
          return;
        }
        pending.push(part);
        const line = Buffer.concat(pending);
        pending = [];
        pendingBytes = 0;
        deliver(line);
        if (closed) return;
        start = nl + 1;
      }
      const rest = chunk.subarray(start);
      if (rest.length > 0) {
        if (pendingBytes + rest.length > MAX_LINE_BYTES) {
          socket.destroy();
          finish("line too long");
          return;
        }
        pending.push(rest);
        pendingBytes += rest.length;
      }
    });
    socket.on("close", () => finish("closed"));
    socket.on("error", (e) => finish(`error: ${e.message}`));

    socket.once("connect", () => {
      socket.removeListener("error", reject);
      resolve({
        send(message) {
          if (!closed) socket.write(JSON.stringify(message) + "\n");
        },
        onMessage(h) {
          messageHandlers.push(h);
        },
        onClose(h) {
          closeHandlers.push(h);
          if (closed) h("closed");
        },
        close() {
          socket.end();
          socket.destroy();
          finish("closed");
        },
      });
    });
    socket.once("error", reject);
  });
}

/** A connection in memory, for tests: what was sent, and a way to deliver or drop. */
export class FakeConnection implements Connection {
  sent: unknown[] = [];
  private messageHandlers: Array<(m: unknown) => void> = [];
  private closeHandlers: Array<(r: string) => void> = [];
  send(message: unknown) {
    this.sent.push(message);
  }
  onMessage(h: (m: unknown) => void) {
    this.messageHandlers.push(h);
  }
  onClose(h: (r: string) => void) {
    this.closeHandlers.push(h);
  }
  close() {
    this.drop("closed");
  }
  receive(message: unknown) {
    for (const h of this.messageHandlers) h(message);
  }
  drop(reason: string) {
    for (const h of this.closeHandlers) h(reason);
  }
}
