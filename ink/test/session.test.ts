import { describe, expect, it } from "vitest";
import { EXIT_REFUSED, FakeConnection, Session } from "../src/index.js";

const size = { columns: 80, rows: 24 };

describe("Session", () => {
  it("says hello, holds each snapshot and acks it, and stamps intents with its cursor", () => {
    const conn = new FakeConnection();
    const s = new Session<string, string>(conn, 7, size);
    s.start();
    expect(conn.sent).toEqual([{ type: "hello", protocol: 1, view: 7, size }]);
    conn.receive({ type: "welcome", protocol: 1, view: 7, cursor: 0 });
    expect(s.state.phase).toBe("ready");
    conn.receive({ type: "snapshot", cursor: 4, view: "Vimes" });
    expect(s.state.view).toBe("Vimes");
    expect(conn.sent.at(-1)).toEqual({ type: "ack", cursor: 4 });
    s.intent("up");
    expect(conn.sent.at(-1)).toEqual({ type: "intent", cursor: 4, intent: "up" });
  });

  it("ends refused with the host's words and the exit code that is not restarted", () => {
    const conn = new FakeConnection();
    const s = new Session<string, string>(conn, 6, size);
    conn.receive({ type: "refuse", wanted: 7, got: 6, message: "view 6; this host speaks view 7" });
    expect(s.state).toMatchObject({ phase: "refused", exitCode: EXIT_REFUSED, message: "view 6; this host speaks view 7" });
    conn.drop("closed");
    expect(s.state.phase).toBe("refused");
  });

  it("keeps every notice, and a lost host is an error exit", () => {
    const conn = new FakeConnection();
    const s = new Session<string, string>(conn, 7, size);
    conn.receive({ type: "notice", text: "one" });
    conn.receive({ type: "notice", text: "two" });
    expect(s.state.notices).toEqual(["one", "two"]);
    conn.drop("closed");
    expect(s.state).toMatchObject({ phase: "closed", exitCode: 1 });
  });

  it("tells listeners of each change", () => {
    const conn = new FakeConnection();
    const s = new Session<string, string>(conn, 7, size);
    let heard = 0;
    const stop = s.subscribe(() => heard++);
    conn.receive({ type: "welcome", protocol: 1, view: 7, cursor: 0 });
    stop();
    conn.receive({ type: "notice", text: "x" });
    expect(heard).toBe(1);
  });
});
