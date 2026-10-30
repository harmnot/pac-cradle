import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { formatMm, sizeLabel } from "./format";
import { paperForLanguages, regionOf } from "./paper";
import { cameraExif, isHeic, readCameraExif, targetSize } from "./prepare-photo";
import { validSize } from "./typed-product";

describe("the paper default", () => {
  it("reads the region a language tag names", () => {
    expect(regionOf("en-US")).toEqual({ kind: "some", value: "US" });
    expect(regionOf("id-ID")).toEqual({ kind: "some", value: "ID" });
    expect(regionOf("en")).toEqual({ kind: "none" });
    expect(regionOf("es-419")).toEqual({ kind: "none" });
    expect(regionOf("not a tag")).toEqual({ kind: "none" });
  });

  it("is Letter in the US, Canada, Mexico and the Philippines, A4 elsewhere", () => {
    expect(paperForLanguages(["en-US", "en"])).toBe("letter");
    expect(paperForLanguages(["fr-CA"])).toBe("letter");
    expect(paperForLanguages(["es-MX"])).toBe("letter");
    expect(paperForLanguages(["fil-PH"])).toBe("letter");
    expect(paperForLanguages(["id-ID", "en-US"])).toBe("a4");
    expect(paperForLanguages(["en-GB"])).toBe("a4");
  });

  it("follows the first language that names a region, and is A4 without one", () => {
    expect(paperForLanguages(["en", "en-US"])).toBe("letter");
    expect(paperForLanguages(["en", "de"])).toBe("a4");
    expect(paperForLanguages([])).toBe("a4");
  });
});

describe("preparing a photo", () => {
  it("knows HEIC by type or by name", () => {
    expect(isHeic({ type: "image/heic", name: "IMG_1.HEIC" })).toBe(true);
    expect(isHeic({ type: "", name: "IMG_1.heif" })).toBe(true);
    expect(isHeic({ type: "image/jpeg", name: "IMG_1.jpg" })).toBe(false);
  });

  it("scales down to 4000 px on the long side, never up", () => {
    expect(targetSize({ width: 8064, height: 6048 })).toEqual({ width: 4000, height: 3000 });
    expect(targetSize({ width: 3024, height: 4032 })).toEqual({ width: 3000, height: 4000 });
    expect(targetSize({ width: 1600, height: 1200 })).toEqual({ width: 1600, height: 1200 });
  });

  it("keeps only make, model and a positive 35 mm focal length", () => {
    expect(
      cameraExif({
        Make: " Apple ",
        Model: "iPhone 15 Pro",
        FocalLengthIn35mmFormat: 24,
        latitude: -6.2,
        GPSLongitude: [106, 49, 0],
      }),
    ).toEqual({
      make: { kind: "some", value: "Apple" },
      model: { kind: "some", value: "iPhone 15 Pro" },
      focalLength35mm: { kind: "some", value: 24 },
    });
    expect(cameraExif({ FocalLengthIn35mmFormat: 0, Make: "" })).toEqual({
      make: { kind: "none" },
      model: { kind: "none" },
      focalLength35mm: { kind: "none" },
    });
    expect(cameraExif(undefined).focalLength35mm).toEqual({ kind: "none" });
  });
});

describe("reading a photo's EXIF with exifr's lite bundle", () => {
  const fixture = (name: string) => new Uint8Array(readFileSync(new URL(`./fixtures/${name}`, import.meta.url)));

  it("finds make, model and focal length, and never the GPS position", async () => {
    const exif = await readCameraExif(fixture("exif-phone.jpg"));
    expect(exif).toEqual({
      make: { kind: "some", value: "Apple" },
      model: { kind: "some", value: "iPhone 15 Pro" },
      focalLength35mm: { kind: "some", value: 26 },
    });
  });

  it("finds nothing in a photo without EXIF", async () => {
    const exif = await readCameraExif(fixture("no-exif.jpg"));
    expect(exif.focalLength35mm).toEqual({ kind: "none" });
    expect(exif.make).toEqual({ kind: "none" });
  });
});

describe("typed sizes", () => {
  it("accept 1 to 1000 mm", () => {
    expect(validSize(52.3)).toBe(true);
    expect(validSize(1)).toBe(true);
    expect(validSize(1000)).toBe(true);
    expect(validSize(0.5)).toBe(false);
    expect(validSize(1000.1)).toBe(false);
    expect(validSize(Number.NaN)).toBe(false);
  });
});

describe("size labels", () => {
  it("read to a tenth of a millimetre", () => {
    expect(formatMm(52.29)).toBe("52.3");
    expect(formatMm(80)).toBe("80");
    expect(sizeLabel({ shape: "round", diameterMm: 52.29 })).toBe("⌀ 52.3 mm");
    expect(sizeLabel({ shape: "box", lengthMm: 80, widthMm: 40.55 })).toBe("80 × 40.6 mm");
  });
});
