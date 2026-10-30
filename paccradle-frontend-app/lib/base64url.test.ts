import { describe, expect, it } from "vitest";
import { decodeBase64url, encodeBase64url } from "./base64url";

const encoder = new TextEncoder();

describe("base64url", () => {
  it("matches RFC 4648's test vectors, unpadded", () => {
    const vectors: Array<[string, string]> = [
      ["", ""],
      ["f", "Zg"],
      ["fo", "Zm8"],
      ["foo", "Zm9v"],
      ["foob", "Zm9vYg"],
      ["fooba", "Zm9vYmE"],
      ["foobar", "Zm9vYmFy"],
    ];
    for (const [text, encoded] of vectors) {
      expect(encodeBase64url(encoder.encode(text))).toBe(encoded);
    }
  });

  it("uses - and _ where base64 has + and /", () => {
    const bytes = new Uint8Array([0xfb, 0xff, 0xbf]);
    expect(encodeBase64url(bytes)).toBe("-_-_");
    expect(decodeBase64url("-_-_")).toEqual({ kind: "some", value: bytes });
  });

  it("round-trips every byte value", () => {
    const bytes = Uint8Array.from({ length: 256 }, (_, index) => index);
    const decoded = decodeBase64url(encodeBase64url(bytes));
    expect(decoded).toEqual({ kind: "some", value: bytes });
  });

  it("refuses text that isn't base64url", () => {
    for (const bad of ["a+b", "a/b", "a b", "Z"]) {
      expect(decodeBase64url(bad), bad).toEqual({ kind: "none" });
    }
  });
});
