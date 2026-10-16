import net from "node:net";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { connect } from "../src/index.js";

let server: net.Server | undefined;
afterEach(() => server?.close());

/** A host stand-in on a real Unix socket: `talk` gets the accepted socket. */
function host(talk: (s: net.Socket) => void): Promise<string> {
  const file = path.join(os.tmpdir(), `tui-kit-ink-${process.pid}-${Math.random().toString(36).slice(2)}.sock`);
  server = net.createServer(talk);
  return new Promise((resolve) => server!.listen(file, () => resolve(file)));
}

describe("connect", () => {
  it("reassembles lines split across reads and sends one line per message", async () => {
    const got: string[] = [];
    const file = await host((s) => {
      s.on("data", (d) => got.push(d.toString()));
      s.write('{"type":"no');
      setTimeout(() => s.write('tice","text":"Vimes"}\n{"type":"shutdown"}\n'), 10);
    });
    const conn = await connect(file);
    const messages: unknown[] = [];
    await new Promise<void>((resolve) =>
      conn.onMessage((m) => {
        messages.push(m);
        if (messages.length === 2) resolve();
      }),
    );
    expect(messages).toEqual([{ type: "notice", text: "Vimes" }, { type: "shutdown" }]);
    conn.send({ type: "bye" });
    await new Promise((r) => setTimeout(r, 20));
    expect(got.join("")).toBe('{"type":"bye"}\n');
    conn.close();
  });

  it("closes on a line that is not JSON", async () => {
    const file = await host((s) => s.write("not json\n"));
    const conn = await connect(file);
    const reason = await new Promise<string>((resolve) => conn.onClose(resolve));
    expect(reason).toBe("invalid json");
  });
});
