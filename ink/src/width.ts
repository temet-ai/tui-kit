import stringWidth from "string-width";

/** Display cells, never `.length`: an emoji or a CJK character takes two, a combining mark none. */
export function width(s: string): number {
  return stringWidth(s);
}

/** `s` cut to at most `cells` cells and padded with spaces to exactly `cells`, as the Rust side's `fit`. */
export function fit(s: string, cells: number): string {
  let out = "";
  let used = 0;
  for (const c of s) {
    const w = stringWidth(c);
    if (used + w > cells) break;
    out += c;
    used += w;
  }
  return out + " ".repeat(cells - used);
}
