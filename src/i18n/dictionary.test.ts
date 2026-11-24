import { describe, expect, it } from "vitest";
import { en, ru, type MessageKey } from "./dictionary";

function placeholders(value: string): string[] {
  return (value.match(/\{(\w+)\}/g) ?? []).map((token) => token).sort();
}

const enKeys = Object.keys(en).sort() as MessageKey[];

describe("dictionary parity", () => {
  it("translates every English key", () => {
    const ruKeys = Object.keys(ru).sort();
    expect(ruKeys).toEqual(enKeys);
  });

  it("has no empty strings in either language", () => {
    for (const key of enKeys) {
      expect(en[key].trim(), `en:${key}`).not.toBe("");
      expect(ru[key].trim(), `ru:${key}`).not.toBe("");
    }
  });

  it("keeps the same interpolation variables in both languages", () => {
    for (const key of enKeys) {
      expect(placeholders(ru[key]), `ru:${key}`).toEqual(placeholders(en[key]));
    }
  });

  it("does not accidentally fall back to English for user-facing prose", () => {
    // Short technical labels are intentionally identical, long sentences are not.
    const identical = enKeys.filter(
      (key) => en[key] === ru[key] && /[a-z]{4}\s+[a-z]{4}\s+[a-z]{4}/i.test(en[key]),
    );
    expect(identical).toEqual([]);
  });
});

describe("kept promises in copy", () => {
  it("never offers a Play button", () => {
    for (const key of enKeys) {
      expect(en[key].toLowerCase()).not.toContain("launch the game");
    }
    expect(en["home.playUnavailable.title"]).toMatch(/No Play button/i);
  });

  it("states that updates require verification", () => {
    expect(en["updates.verify.body"]).toContain("Compatibility verification is required");
  });
});
