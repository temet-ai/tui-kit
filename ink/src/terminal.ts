/**
 * What leaving the terminal always writes, the same sequence as tui-kit-ipc's
 * tty::RESTORE: mouse, focus and paste reports off, attributes reset, the
 * alternate screen left, the cursor shown. Safe to repeat.
 */
export const RESTORE =
  "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l\x1b[0m\x1b[?1049l\x1b[?25h";

/**
 * Makes `leave(code, message)` the one way out: on every path (a normal end, an
 * uncaught error, a signal) it runs `cleanup` once (unmount the UI), restores
 * the terminal, prints the message after it, and exits with the code. A
 * non-zero code makes the host's supervisor restart the renderer.
 */
export function oneWayOut(
  cleanup: () => void,
  out: { write(s: string): unknown } = process.stdout,
): (code: number, message?: string) => never {
  let leaving = false;
  const leave = (code: number, message?: string): never => {
    if (!leaving) {
      leaving = true;
      try {
        cleanup();
      } catch {}
      out.write(RESTORE);
      if (message) process.stderr.write(`${message}\n`);
    }
    process.exit(code);
  };
  process.on("uncaughtException", (e) => leave(1, `renderer error: ${e.message}`));
  process.on("unhandledRejection", (e) => leave(1, `renderer error: ${String(e)}`));
  for (const sig of ["SIGTERM", "SIGHUP", "SIGINT"] as const) process.on(sig, () => leave(1));
  return leave;
}
