# Vision accuracy spike (M0-07 to M0-11)

This folder holds the test photos, their true sizes and the accuracy report for the
Week 1 go/no-go check (CLAUDE.md §5.6, Gate 0 in MILESTONES.md).

```text
spike/
├── README.md     this file
├── truth.csv     one line per photo: the product's caliper sizes   (committed)
├── photos/       your photos                                       (git-ignored)
├── out/          debug images from the measure tool                (git-ignored)
└── REPORT.md     written by the accuracy tool                      (commit it)
```

## 1. Take the photos (M0-07, you)

Pick 10 objects: a flat box, a jar, a perfume bottle, a tube, a round item, a white item,
a glass item, a dark item and 2 of your choice. Measure each with calipers.

For **each object**, take these photos with **2 phones**:

| Mode  | How |
| ----- | --- |
| photo | Product in the middle of the paper, at least 2 cm from the edges. |
| trace | Draw around the product with a pen held upright, remove the product, photograph the paper. |

Do both on **A4** and on **US Letter** if you have both. Capture rules (CLAUDE.md §5.1):

- Paper on a flat, **darker** surface; all 4 corners in the photo.
- Phone **high** (≈ 50 cm) and **level**, **2× zoom**.
- Soft light from the side or a window, no hard shadows.
- White or glass product: use coloured paper or trace mode.

Save the photos as JPEG in `photos/`. A name like `jar-photo-a4-iphone.jpg` helps.
HEIC photos: export them as JPEG first.

## 2. Fill in truth.csv

One line per photo. Leave a number empty when it doesn't apply. No commas inside a cell.

| Column             | Example                    | Notes                                        |
| ------------------ | -------------------------- | -------------------------------------------- |
| `photo`            | `jar-photo-a4-iphone.jpg`  | file name in `photos/`                       |
| `object`           | `candle jar`               |                                              |
| `mode`             | `photo` or `trace`         |                                              |
| `paper`            | `a4` or `letter`           |                                              |
| `true_l_mm`        | `80.2`                     | box-like products: long side                 |
| `true_w_mm`        | `30.1`                     | box-like products: short side                |
| `true_diameter_mm` | `52.3`                     | round products: fill this instead of L and W |
| `height_mm`        | `95`                       | sorts photo mode into "low" (≤ 20) or "tall" |
| `phone`            | `iphone 15`                |                                              |
| `notes`            | `window light`             |                                              |

## 3. Run the tools (from `paccradle-backend/`)

Without OpenCV on the Mac, run everything inside the Docker toolchain image:

```bash
scripts/in-docker.sh cargo run -p vision --release --bin measure -- crates/vision/spike/photos/jar-photo-a4-iphone.jpg --paper a4 --mode photo --debug-dir crates/vision/spike/out/jar
```

`measure` prints the result as JSON (CLAUDE.md §5.5) and, with `--debug-dir`, writes what it
saw: `1-paper.jpg` (paper found), `2-flattened.jpg`, `3-mask.png`, `4-outline.jpg`.
Trace mode takes `--pen-width-mm` (default 0.5).

Both tools read photos the way a capture job does (M1-C1): JPEG, PNG or WebP only, at least
1000 px on the long side and at most 40 million pixels, turned upright by their EXIF
orientation. The accuracy report runs `vision::run`, the job's own entry point.

```bash
scripts/in-docker.sh cargo run -p vision --release --bin accuracy
```

`accuracy` measures every photo in `truth.csv` and writes `REPORT.md`: median and P90 error
per group against the §5.6 targets, and one line per photo.

## Known limits of the spike

- No EXIF focal length yet (only needed for pose correction in v1, M3-03).
- Photo mode can't see height: tall products read too big from above (CLAUDE.md §5.2).
- On generated test photos the error is about ±0.25 mm (photo mode reads slightly big,
  trace mode slightly small). Calibrate on real photos before changing anything.
