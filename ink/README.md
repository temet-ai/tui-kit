# tui-kit-ink

For drawing a Rust terminal app's screen with [Ink](https://github.com/vadimdemedes/ink)
(React in the terminal). The Rust program uses [`tui-kit-ipc`](../crates/tui-kit-ipc): it
starts your Ink app as a child process, gives it the terminal, and passes it the path of
a socket in an environment variable you choose. This package is the Ink app's end of that
socket. It keeps the newest view the Rust program sent, sends back what the user pressed,
and puts the terminal back however the app exits.

You do not run the Ink app yourself: the Rust program starts it, and restarts it if it
crashes. Both packages are in [tui-kit](https://github.com/temet-ai/tui-kit) (MIT).

## What you need

Node 22 or later, Ink 8 and React 19 in your app (this package imports neither, so your
app keeps one copy of each), and a Rust program using `tui-kit-ipc`. Not on npm; from a
checkout of tui-kit:

```json
"dependencies": { "tui-kit-ink": "file:../tui-kit/ink" }
```

## A renderer

`src/main.tsx`, for a Rust program whose view is a string and whose input is a string:

```tsx
// These two imports come first. ES modules run every import before the code below,
// so Node's fetch is already hidden when Ink loads, and restoreFetch() puts it back after.
import "tui-kit-ink/require-shim";
import { restoreFetch } from "tui-kit-ink/no-fetch";
import { render, Text, useInput } from "ink";
import { useSyncExternalStore } from "react";
import { connect, oneWayOut, Session } from "tui-kit-ink";

restoreFetch();
type View = string;   // must match the Rust program's App::View
type Input = string;  // and its App::Intent

let ui: ReturnType<typeof render> | undefined;
const leave = oneWayOut(() => ui?.unmount());
const conn = await connect(process.env.MYAPP_IPC_SOCKET!);   // the variable the Rust side names
const session = new Session<View, Input>(conn, 1, { columns: process.stdout.columns, rows: process.stdout.rows });
session.subscribe(() => {
  const s = session.state;
  if (s.phase === "closed" || s.phase === "refused") leave(s.exitCode ?? 1, s.message);
});

function App() {
  const state = useSyncExternalStore((f) => session.subscribe(f), () => session.state);
  useInput((input) => session.intent(input));
  return <Text>{state.view ?? ""}</Text>;
}

ui = render(<App />, { alternateScreen: true, exitOnCtrlC: false });
session.start();
```

The Rust program sets `MYAPP_IPC_SOCKET` (whatever name it passed to `Options::new`)
before starting this file; run by hand, the variable is missing and the app should exit
with code 2. The `1` is the view version; it must equal the number the Rust program
passes to `Options::new`, or the Rust side refuses this renderer. `subscribe` returns a
function that unsubscribes. The Rust side's README has the matching Rust code.

`session.state` holds:

| Field | Meaning |
|---|---|
| `phase` | `connecting` until the Rust side answers, then `ready`; `refused` or `closed` at the end |
| `view` | the newest view received (it is acknowledged for you, which lets the next one come) |
| `cursor` | that view's number; `session.intent` sends it with each input |
| `notices` | status lines the Rust side sent, oldest first |
| `exitCode`, `message` | how to exit and what to print, once `phase` is `refused` or `closed` |

Also on `Session`: `resize(size)` when the terminal changes, `log(level, text)` to write to
the Rust program's log (the renderer cannot print: it owns the screen), `bye()` to quit.

## The other exports

- `oneWayOut(cleanup)` returns `leave(code, message?)`. It is called on a normal end, an
  uncaught error or a signal: it runs `cleanup` (unmount), restores the terminal, prints
  the message, and calls `process.exit(code)`. Codes: 0 normal, 1 error (the Rust side
  restarts the renderer), 2 no socket variable, 3 refused (not restarted).
- `width(s: string): number` and `fit(s: string, cells: number): string` count display
  cells as the Rust side does: an emoji or a CJK character takes two.
- `FakeConnection`: an in-memory connection, for testing a renderer without the Rust side.

## Bundling

The Rust program starts one `.js` file. Make it with esbuild:

```sh
esbuild src/main.tsx --bundle --platform=node --format=esm --target=node22 \
  --alias:react-devtools-core=tui-kit-ink/devtools-stub \
  --define:process.env.NODE_ENV=\"production\" --outfile=dist/renderer.js
```

What the three helpers are for: `require-shim` lets CommonJS packages inside the ESM bundle
call `require`. `no-fetch` hides Node's `fetch` while Ink's layout engine loads; otherwise
it loads Node's HTTP client and start-up takes 30 to 45 ms longer. `devtools-stub` stands
in for React DevTools, which Ink imports but a shipped renderer never uses.

## Tests

`npm install && npm run typecheck && npm test`.
