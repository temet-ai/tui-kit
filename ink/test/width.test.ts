import { describe, expect, it } from "vitest";
import { fit, width } from "../src/index.js";

// The same cases as the Rust side's width tests: both renderers cut the same.
describe("width and fit", () => {
  it("counts display cells", () => {
    expect(width("Vimes")).toBe(5);
    expect(width("🦉")).toBe(2);
    expect(width("安克")).toBe(4);
  });

  it("cuts by cells and never splits a wide character", () => {
    expect(fit("安克摩波克", 5)).toBe("安克 ");
    expect(fit("Vimes", 7)).toBe("Vimes  ");
    expect(width(fit("🦉🦉🦉", 5))).toBe(5);
  });
});
