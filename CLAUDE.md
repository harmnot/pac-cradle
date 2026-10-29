# Plan: Paccradle — Photo → Box Insert & Partition Dieline Generator

**Author:** Claude (Opus 5.5) · **Date:** 2026-10-26 · **Status:** plan, before validation
**Working name:** paccradle (change it freely)
**Stack you chose:** React + Next.js + Three.js (frontend, no Tailwind) · Rust + Axum (backend) · OpenCV + backup AI (vision) · Paddle (payments)

> **Execution tracker: [MILESTONES.md](MILESTONES.md).** It holds the milestones, tasks, open decisions and progress log for this plan. At the start of each session, read its "Current focus" and work from there. When a task is done, tick it there and update its logs.

> Prices, competitor facts and legal dates below were checked on 2026-10-26. Numbers marked **(estimate)** are mine, not from a source. Check them again before you rely on them.

---

## Contents

0. [One-page summary](#0-one-page-summary)
1. [Market check](#1-market-check-26-oct-2026)
2. [Who pays and why](#2-who-pays-and-why)
3. [User flow](#3-user-flow)
4. [What makes it unique](#4-what-makes-it-unique-feature-ideas)
5. [Vision pipeline (photo → exact outline)](#5-vision-pipeline-photo--exact-outline)
6. [Dieline engine (Rust geometry core)](#6-dieline-engine-rust-geometry-core)
7. [2D editor & 3D fold preview](#7-2d-editor--3d-fold-preview-react--threejs)
8. [System architecture](#8-system-architecture)
9. [Payments with Paddle](#9-payments-with-paddle)
10. [Pricing](#10-pricing)
11. [Getting customers without cold sales](#11-getting-customers-without-cold-sales)
12. [Roadmap](#12-roadmap-solo-week-by-week)
13. [Metrics and go/no-go rules](#13-metrics-and-gono-go-rules)
14. [Risks](#14-risks-and-mitigations)
15. [Decisions for you](#15-decisions-for-you)
16. [Sources](#16-sources)

---

## 0. One-page summary

**What it does:** The customer puts a product on A4 or US Letter paper and takes one photo. The app finds the paper, measures the product, and generates a **cut-ready insert** that holds it: a partition grid, a cutout card or a raised cradle. It also generates a **matching outer box**. The customer edits it in 2D, watches it fold in 3D, checks the fit, and exports SVG/PDF/DXF files for a printer, laser, CNC table or Cricut.

**Who pays:**

- Small product brands (perfume, cosmetics, candles, skincare, gift sets).
- Freelance packaging designers.
- Small print shops.
- Makers with Cricut, laser or CNC machines.

**The gap:** Many dieline generators exist (Pacdora, DielineMaker, CEF Box, PackagingDieline.com). Several photo→outline tools exist for foam and 3D-printed bins (toolKaiser, TooltraceAI). I found **no tool that goes from a product photo to a folding-cardboard insert plus a matching box with a fit check** [1–10].

**How money comes in:** Paddle checkout, either **$10 per export** or **Pro at $12/month**. Paddle is the merchant of record (it handles card payments, tax and refunds) and pays you out [11–13].

**How customers find you:**

- Google searches for insert templates.
- Short demo videos.
- Maker and small-brand communities.
- Share links and watermarked unboxing videos.

You never send cold messages.

**MVP effort:** about 8–10 weeks solo, plus 2 weeks of validation first **(estimate)**.

**Top 3 risks:**

1. Measuring tall products from a top photo is wrong by up to 30% unless corrected (§5.2).
2. Paddle approval, and payouts to Indonesia, are not confirmed (§9.1).
3. Pacdora could copy the idea (§14).

---

## 1. Market check (26 Oct 2026)

| Tool                                                         | What it does                                                                                                                                                  | What it does NOT do                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **Pacdora** [4][5][6]                                        | Big dieline library, 3D mockups, AI artwork mapped onto dielines (launched Apr 2026), text-to-dieline. Paid plans about $12–29/month on third-party listings. | No photo → measured insert found                         |
| **CEF Box** [1]                                              | Free 4-compartment insert generator. You type product sizes and it exports PDF/DXF.                                                                           | Standard grids only, no photo, no product-shaped cutouts |
| **PackagingDieline.com** [2]                                 | Parametric "display box with divider" and "frame insert"                                                                                                      | No photo, no custom cutouts                              |
| **DielineMaker** [3]                                         | Parametric dielines, exports PDF/AI/SVG/DXF/CF2. Downloads need Pro.                                                                                          | No photo                                                 |
| **QinPrinting, Packpaa, others** [10]                        | Free box dieline generators (length × width × height → PDF)                                                                                                   | Boxes only, no inserts from photos                       |
| **toolKaiser, TooltraceAI, MLBench DXF generator** [7][8][9] | Photo → tool outline → **foam** insert / 3D-printed Gridfinity bin                                                                                            | Foam and 3D-print bins, not folding cardboard dielines   |

**Conclusion:**

- I found nobody doing the full chain: photo → measured product → cardboard insert → matching box → 3D fit check → production files.
- A 3D fold preview alone is **not** unique, because Pacdora has 3D previews.
- The unique part is the **measuring and fitting**.
- Pacdora is the most likely to add this, so move fast and own the "inserts for small brands and makers" niche.

---

## 2. Who pays and why

| Segment                                                                              | Their job                                                        | What they export                                       | Willingness to pay     |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------- | ------------------------------------------------------ | ---------------------- |
| Small product brands (perfume, cosmetics, candles, tea, jewelry, gadget accessories) | Make products sit tight and look premium in a gift or mailer box | PDF/DXF for their printer, plus a paper test print     | Medium: pay per export |
| Gift set and subscription box sellers                                                | Fit **several different items** in one box                       | Kit insert + box                                       | Medium–high: Pro       |
| Freelance packaging and graphic designers                                            | Fast structural design for clients, plus 3D renders to show them | PDF/DXF + PNG/MP4 renders                              | Medium–high: Pro       |
| Small print and packaging shops                                                      | Quick insert designs for quotes and sample-cutting tables        | DXF/PDF (later CF2)                                    | High: Studio           |
| Makers (Cricut, Silhouette, laser, CNC)                                              | Short runs at home                                               | SVG with score lines, laid out to the cutting-mat size | Low–medium: per export |

**Lead with small brands and designers.** Makers bring traffic and word of mouth but pay less.

---

## 3. User flow

```text
 ┌──────────────┐   ┌──────────────────┐   ┌───────────────────┐   ┌───────────────────┐
 │ 1. Capture   │──▶│ 2. Detect & check │──▶│ 3. Pick insert,   │──▶│ 4. Edit 2D ⇄ 3D   │
 │ photo/trace/ │   │ paper + product,  │   │ material & box    │   │ fold, fit check,  │
 │ type sizes   │   │ size, height      │   │ (AI suggests one) │   │ warnings          │
 └──────────────┘   └──────────────────┘   └───────────────────┘   └─────────┬─────────┘
                                                                              │
 ┌──────────────┐   ┌──────────────────┐   ┌───────────────────┐              ▼
 │ 7. Share link│◀──│ 6. Download files │◀──│ 5. Export ▸ Paddle │◀── free preview + test print
 │ / unboxing   │   │ SVG/PDF/DXF/mockup│   │ ($10 or Pro)       │
 └──────────────┘   └──────────────────┘   └───────────────────┘
```

**Step by step:**

1. **Choose how to capture.**
    - (a) **Photo on paper.** Fastest.
    - (b) **Trace on paper.** Draw around the product with a pen, remove it, then photograph the paper. Most accurate (§5.2).
    - (c) **Type the sizes.** No camera needed.
    - (d) **Continue on phone.** Scan a QR code on the desktop and take the photo with the phone.
2. **Choose the paper.** A4 or US Letter, defaulted by country (Letter for the US, Canada, Mexico and the Philippines; A4 elsewhere). The user can switch.
3. **Take the photo with guidance.**
    - The whole paper is visible and the product sits in the middle.
    - The phone is high and level, with 2× zoom.
    - Use soft light.
    - White product? Use colored paper.
4. **Detection (1–3 s).**
    - The app shows the paper outline, the product outline, its size (L × W or diameter) and a confidence score.
    - The user can drag outline points or tap the product if the app picked the wrong thing.
5. **Height.** Type it from a ruler, or take a lay-down photo with the product on its side (v1).
6. **Quantity and kit.** For example "6 of this bottle", or "add another product" for gift sets.
7. **Insert style.** Visual cards: partition grid, cutout card, cradle platform (later: pocket tray, neck holder). The AI recommends one.
8. **Material and box.** Paperboard, chipboard, or E/B/C-flute corrugated. Then a mailer box, a shipping box or a tuck-end carton, sized automatically. The app shows the empty-space %.
9. **Editor.** A 2D dieline (cut = solid line, crease = dashed line, with dimensions) next to a 3D preview with a fold slider, isometric view and fit-check colors. A warnings panel lists problems such as "hole too close to fold".
10. **Export.**
    - Free: a watermarked PNG and a **watermarked print-at-home test PDF**.
    - Paid: production files (SVG/PDF/DXF), maker presets and a spec sheet.
11. **Download, share and come back.**
    - The design is saved in "My designs".
    - The user can send a view-only 3D share link to a client or printer.
    - Pro users can export an unboxing video.

---

## 4. What makes it unique (feature ideas)

| #   | Idea                                                                                                                                             | Why it matters                                                                                                                                                                                                                                          | Phase              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| U1  | **Trace mode**: draw around the product with a pen on the paper, then photograph the paper without the product                                   | The pen line sits flat on the table, so the photo has **no parallax error** (§5.2). Gives the most accurate fit with the simplest method.                                                                                                               | MVP                |
| U2  | **Fit check**: product shown inside the insert in 3D, colored green, yellow or red by clearance                                                  | Customers see "it fits" before paying a printer                                                                                                                                                                                                         | MVP (2D), MVP (3D) |
| U3  | **Print-at-home test**: insert tiled across A4/Letter sheets with alignment marks and folding steps                                              | A free paper prototype in 10 minutes, which builds trust and drives sign-ups                                                                                                                                                                            | MVP                |
| U4  | **Auto outer box** (mailer / shipping box / tuck-end carton) sized around the insert                                                             | One step gives a full packaging set, not only the insert                                                                                                                                                                                                | MVP                |
| U5  | **Kit builder**: several different products → automatic layout → one insert + one box                                                            | Gift sets are where brands spend money. No competitor found doing this from photos.                                                                                                                                                                     | v1                 |
| U6  | **Live camera guide**: paper detected live with a green frame, plus a phone-level indicator using the gyroscope                                  | Fewer bad photos and better accuracy. Feels magical.                                                                                                                                                                                                    | v1                 |
| U7  | **Lay-down photo + pose correction** (camera height from the 4 paper corners)                                                                    | Accurate sizes for tall bottles from photos (§5.2)                                                                                                                                                                                                      | v1                 |
| U8  | **Maker presets**: Cricut/Silhouette SVG with a score layer, laid out on 12×12″ / 12×24″ mats; kerf (laser cut-width) compensation for laser/CNC | Opens the large maker community, which brings traffic and shares                                                                                                                                                                                        | v1                 |
| U9  | **Unboxing animation export** (MP4/WebM/GIF), watermarked on free                                                                                | Brands post it on social media; the watermark brings new users                                                                                                                                                                                          | v1                 |
| U10 | **Printer spec sheet** (PDF): sizes, material, total cut and crease length, how many inserts fit on a standard sheet                             | Makes ordering from a printer easy and looks professional                                                                                                                                                                                               | v1                 |
| U11 | **Empty-space ratio + board area/weight** next to the box                                                                                        | The EU packaging rules (PPWR) apply from 12 Aug 2026. A **50% empty-space cap** for e-commerce packaging comes on 1 Jan 2030 (or later, if the EU's detailed rules come late), and filler material counts as empty space [17]. Brands want this number. | v1                 |
| U12 | **AI insert advisor**: suggests style, board type and clearance from the photo                                                                   | Beginners don't know "E-flute" or "partition". The AI explains in one line.                                                                                                                                                                             | v1                 |
| U13 | **Pro outputs**: CF2 (a file format die makers use), PDF with named spot colors for cut and crease lines                                         | Pulls in print shops (Studio plan)                                                                                                                                                                                                                      | v2                 |
| U14 | **Embeddable widget for printers**: printers put the generator on their own site                                                                 | Inbound partners, no cold sales: a "for printers" page only                                                                                                                                                                                             | v2                 |
| U15 | **Layered cardboard nest**: 3–5 cut layers glued together, like a foam insert but plastic-free                                                   | Unique, eco-friendly, laser-friendly                                                                                                                                                                                                                    | v2                 |

---

## 5. Vision pipeline (photo → exact outline)

### 5.1 Capture rules (shown in the UI)

- Put the paper on a flat, darker surface so it stands out. Make sure all 4 corners are visible.
- Put the product in the middle of the paper, at least 2 cm from the edges.
- Hold the phone **high** (≈50 cm), **level**, and use **2× zoom**. This makes the photo closer to a straight top view.
- Use soft light from the side or a window. Avoid hard shadows.
- For white or transparent products, use **colored A4/Letter paper** or Trace mode.
- Recommend one photo, at most 4000 px on the long edge.

### 5.2 The parallax problem (must solve)

From above, the **top of a tall object looks bigger** than it is, because it is closer to the camera.

> Example: phone 40 cm above the table, bottle 10 cm tall. The top looks 40 / (40 − 10) = **1.33× bigger**, so a 5 cm cap measures ~6.7 cm. Even at 2× zoom from 80 cm: 80 / 70 = 1.14×, still about 7 mm too big on 5 cm.

A plain top photo is therefore fine for **flat or low** products and wrong for **tall** ones. There are three fixes:

| Fix                               | How                                                                                                                                                                         | Accuracy        | Phase |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ----- |
| **A. Trace mode** (U1)            | The pen outline lies on the paper plane, so there is no parallax. Detect the line, take its inner edge, and add back half the pen width (setting, default 0.5 mm).          | Best            | MVP   |
| **B. Lay down + type height**     | Bottles lie on their side, which lowers the height. The user types the height from a ruler. Warn when height ÷ camera distance is too big.                                  | Good for most   | MVP   |
| **C. Pose-aware correction** (U7) | Estimate the camera position from the 4 paper corners and the camera's focal length from the photo's EXIF data. Then project the product outline back onto its true height. | Good, automatic | v1    |

### 5.3 Level 1: classic OpenCV (free, fast)

Use the Rust `opencv` crate (v0.101.1, released 2026-10-09). It supports OpenCV 4.x/5.x and needs clang to build [22].

1. **Load.** Fix EXIF rotation and read the focal-length EXIF tags. Make a working copy at about 2000 px on the long edge.
2. **Find the paper.**
    - Grayscale → blur → Canny edges (or adaptive threshold) → `findContours` → `approxPolyDP`.
    - Keep the biggest convex 4-corner shape that covers >20% of the photo.
    - Refine the corners to sub-pixel accuracy with `cornerSubPix`.
3. **Flatten.** `getPerspectiveTransform` + `warpPerspective` onto a top-down view at **10 px/mm**: A4 becomes 2100 × 2970 px, Letter 2159 × 2794 px. Now **1 px = 0.1 mm**.
4. **Cut out the product.**
    - Look only inside the paper, ignoring a 3 mm band at the edges.
    - Mark pixels that differ from the paper color (Lab color distance + adaptive threshold).
    - Clean up the mask with morphology, refine it with GrabCut, and keep the largest blob (or all blobs in kit mode).
5. **Reduce shadows.** Divide by a blurred background to even out the light. Shadows are darker but not colorful, so drop dark low-saturation edge areas.
6. **Measure.**
    - `minAreaRect` gives L × W and rotation.
    - Circle/ellipse fit for round products.
    - Also compute the convex hull and area.
7. **Clean the outline.** Simplify (`approxPolyDP`, ε ≈ 0.2 mm), smooth it, and store it in **millimetres**.
8. **Score confidence (0–1).**
    - Higher when the outline is smooth and solid, the edge is strong, and the product sits fully inside the paper.
    - Lower when the outline touches the paper edge, several blobs are unexpected, or shadows are likely.

### 5.4 Backup AI (only when Level 1 is unsure)

```text
L1 OpenCV ── confidence ≥ 0.85 ──▶ done
    │ 0.60–0.85
    ▼
L2 SAM 2 (Candle, on server CPU) with a box prompt from L1 ── ok ──▶ done
    │ < 0.60 or failed
    ▼
L3 AI helper (Hugging Face → Gemini free tier → fal.ai):
   "which object is the product? what's wrong with this photo?"
    → returns box/point prompt + tips ──▶ L2 again ── ok ──▶ done
    │ still bad
    ▼
Ask the user: tap the product, or switch to Trace mode
```

**L2: open-source segmentation**

- **SAM 2** (tiny 38.9M or small 46M parameters). Its checkpoints are **Apache 2.0** licensed [18]. Run it through Pure Rust with the `Candle` crate. 2.0 is still a release candidate, so **pin the exact version** [23].
- **BiRefNet** (repo is MIT; check the license of the weights) is an alternative. It also has a lite version that third-party tools run in the browser [20].
- **Do not use RMBG-2.0.** It is CC BY-NC 4.0, so commercial use needs a paid license from BRIA [21].
- **SAM 3** uses a new custom "SAM License". Read it before using it [19].

**L3: vision-language model, as a helper only, never as the ruler**

- **Jobs:**
    - Pick the product when there are several objects.
    - Spot glass, reflections, shadows, or a product touching the paper edge.
    - Return a **box/point prompt** for SAM 2. Each provider returns boxes in its own format (see "From Rust"), so one adapter turns them into pixels of the flattened paper image. Give it a unit test per provider, using a test image with a known box.
    - Recommend an insert style and board type.
    - Write a one-line tip for the user.
- **Model type:** a **vision-language model**: it looks at the photo and answers in JSON. Image generators can't do this job. Stable Diffusion line-art checkpoints and ControlNet draw new pictures; they can't say where the product is.
- **Provider chain** (decided 2026-10-28, DEC-13). Claude's API is not used (too expensive). The worker tries each step in order. It moves to the next step when a step fails, times out (`AI_STEP_TIMEOUT_SECONDS`, default 8 s) or runs out of quota.

| Step | Provider                                                                   | Model (check again when building M3-02)                                                     | Cost                                                                                                                                         | Notes                                                                                                                                                                                                                                                       |
| ---- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | **Hugging Face Inference Providers**, routed with `HUGGINGFACE_TOKEN` [24] | `Qwen/Qwen3-VL-8B-Instruct` (Apache 2.0). Spare: `Qwen/Qwen2.5-VL-7B-Instruct` (Apache 2.0) | Paid per call at the provider's rate; HF adds nothing. **Free HF accounts get no monthly credits** (checked 2026-10-28): buy credits, or use PRO, which includes $2/month | On 2026-10-28 both models were live on one provider only (featherless-ai). Expect outages; steps 2 and 3 cover them.                                                                                                                                        |
| 2    | **Gemini API free tier**, on your Indonesian Google account [27]           | `GEMINI_MODEL` (default `gemini-2.5-flash-lite`)                                             | $0 within the daily quota. HTTP 429 means the quota is used up: go to step 3                                                                 | **Skipped for visitors in the EEA, the UK and Switzerland.** Google allows only paid Gemini for apps serving users there, wherever the developer is based [27]. Free-tier inputs may be used to improve Google's products, and human reviewers may read them [27]. |
| 3    | **fal.ai**, with `FAL_KEY` [28]                                            | `fal-ai/moondream3-preview/detect` for the box; `fal-ai/moondream3-preview/query` for the tip | Paid per call; measure it in M3-02                                                                                                           | Also the backup for EEA/UK/Swiss visitors when step 1 fails. Median ≈1.4 s for detect (fal's numbers, week of 19 Oct 2026).                                                                                                                                   |

- **The site is global.** The visitor's region comes from the country we already use for the paper default (§3 step 2). **When the country is unknown, treat the visitor as EEA/UK/Swiss** and skip step 2.
- **What we send:** only the flattened paper image from L1 step 3 (the paper, not the room around it), scaled to about 1000 px on the long edge. That means fewer tokens and less personal data. If L1 found no paper, send the whole photo at the same size.
- **From Rust:** none of the three has an official Rust SDK, so call them over HTTPS with `reqwest`.
    - **Hugging Face:** `POST https://router.huggingface.co/v1/chat/completions` (the OpenAI chat format) with `Authorization: Bearer <token>`. Send the image as a base64 data URL in an `image_url` part. Ask for JSON with `response_format` where the provider supports it; otherwise pull the JSON out of the text and validate it. Qwen3-VL returns boxes on a 0–1000 scale of the image; Qwen2.5-VL returns pixels of the image size the model actually used.
    - **Gemini:** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent` with header `x-goog-api-key`. Send the image as `inline_data`, and set `generationConfig.responseMimeType` to `application/json` with a response schema. Boxes come back as `box_2d: [ymin, xmin, ymax, xmax]` on a 0–1000 scale. Check `promptFeedback.blockReason` and the candidate's `finishReason` before reading the answer.
    - **fal.ai:** `POST https://fal.run/{endpoint}` with `Authorization: Key <FAL_KEY>`, and the image as a data URI in `image_url`. Use the queue API (`queue.fal.run`) if calls get slow. Detect returns `objects` with `x_min`, `y_min`, `x_max`, `y_max` on a 0–1 scale.
- **Cost per call (estimate):**
    - A 1000 px image is about 1,000–1,500 image tokens. Plus the prompt, that's about 2,000 input tokens and 300 output tokens.
    - Log each call's usage: `usage` (Hugging Face), `usageMetadata` (Gemini), `usage_info` (fal.ai). Check the monthly total in M3-02 before you decide on limits.
- **Privacy:** tell users that unclear photos may be sent to an AI provider: Hugging Face and its inference partner, Google (Gemini) or fal.ai. Don't send photos from users who opt out.

**User retries.** L3 runs on its own when L1 and L2 are unsure (diagram above). Users can also ask for a new result when they think the outline is wrong:

- Retries 1 and 2 run only the open-source steps on our server (L1 and L2), using the user's tap point if they gave one.
- Retry 3 is the last automatic one. It adds the L3 AI helper.
- If the user still isn't happy, the app suggests a better photo (§5.1 capture rules), Trace mode (draw around the product, §5.2 fix A), or dragging the outline points by hand (§3 step 4).

### 5.5 Vision output (JSON)

```json
{
    "paper": {
        "size": "A4",
        "corners_px": [
            [412, 310],
            [3655, 288],
            [3712, 2890],
            [380, 2925]
        ],
        "px_per_mm": 10.0
    },
    "products": [
        {
            "id": "p1",
            "shape": "round",
            "confidence": 0.91,
            "bbox_mm": { "l": 52.4, "w": 52.1 },
            "diameter_mm": 52.2,
            "contour_mm": [[0.0, 26.1], [1.2, 32.9], "..."],
            "height_mm": null,
            "height_source": "user"
        }
    ],
    "warnings": [{ "kind": "tall_product_looks_bigger" }],
    "level_used": "L1"
}
```

Warnings are codes (`kind`, plus any numbers) that the editor translates; failures carry a stable error code such as `CAPTURE_PAPER_NOT_FOUND` (M1-C1). The output also gives each product's `angle_deg`, `center_mm` and `area_mm2`, and the capture `mode`.

### 5.6 Accuracy targets and test plan

**Test set:**

- 30 real objects: boxes, jars, perfume bottles, tubes, glass, white and dark items.
- 3 phones: an iPhone, a Samsung and a cheap Android.
- A4 and Letter paper, 2 lighting setups.
- Measure the true size with calipers.

| Mode                                                   | Target error, P90 | Target error, median |
| ------------------------------------------------------ | ----------------- | -------------------- |
| Trace mode                                             | ≤ 1.0 mm          | ≤ 0.5 mm             |
| Photo, flat or low items (≤ 20 mm tall)                | ≤ 1.5 mm          | ≤ 0.8 mm             |
| Photo, tall items with lay-down / pose correction (v1) | ≤ 2.0 mm          | ≤ 1.0 mm             |

These targets are design goals, not measured results. Run this test set in **Week 1** as a go/no-go step (§12).

---

## 6. Dieline engine (Rust geometry core)

### 6.1 One engine, two places

Write the geometry once, as the Rust crate `insert_core`, and compile it twice:

- **Native**, inside Axum: final exports (SVG/PDF/DXF/CF2).
- **WASM** (wasm-bindgen), in the browser: instant preview while the user drags sliders.

The export writers (DXF/PDF/CF2) are compiled **only into the server build**. A Cargo feature flag controls this. The browser build is a thin wrapper crate, `insert_core_wasm` (M1-A10), that never turns that flag on.

Libraries:

- **`i_overlay`** for polygon offsets and booleans (picked in M1-A2: pure Rust, so it builds for WASM, and it's the engine inside `geo`). The C++ Clipper2 binding can't build for WASM.
- No `kurbo`: paths are only lines and circular arcs, and the crate flattens arcs with a few lines of its own math (M1-A2).
- For the exports: SVG, PDF and DXF (R12) are all written by hand (M1-B1 to B3). They need only lines, colours, layers and built-in-font text, so no crate (the vanilla-Rust rule). The free print-at-home test PDF (M1-B4) uses the same PDF writer. The free PNG preview (M1-B5) is the exception to "no crate": it's drawn with `tiny-skia` and `ttf-parser`, with the Geist font bundled, server only.

```rust
// simplified data model (all units mm)
struct Design { material: Material, products: Vec<Product>, insert: InsertSpec, outer_box: Option<BoxSpec> }
struct Material { kind: Board, thickness_mm: f64 }            // Board: Paperboard | Chipboard | Flute(E|B|C)
struct Product { id: String, contour: Polygon, height_mm: f64, qty: u32 }
enum InsertSpec { PartitionGrid{..}, CutoutCard{..}, CradlePlatform{..}, PocketTray{..}, NeckHolder{..} }
struct Piece { outline: Path, holes: Vec<Path>, creases: Vec<Path>, perfs: Vec<Path>, hinges: Vec<Hinge> }
struct Hinge { panel_a: usize, panel_b: usize, axis: Line, fold_deg: f64, valley: bool }
```

### 6.2 Insert styles

| Style                                                                                 | Best for                                     | Main settings                                                                                                                                                                              | Phase |
| ------------------------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| **Partition grid** (slotted dividers, the FEFCO 09xx "interior fittings" family [16]) | Several identical items: bottles, jars       | Rows × columns from the product size + clearance (cells stretch to fill a fixed box); divider height; slot width = board thickness + tolerance; slot depth = half the height + 0.5 mm; optional outer frame and finger notches | MVP   |
| **Cutout card** (flat card with product-shaped holes)                                 | Lipsticks, tubes, jewelry, small electronics | Card = box inside size − clearance; hole = product outline + clearance; finger notches (side and position settable, default middle of a long side); product positions and row alignment settable, default centred | MVP   |
| **Cradle platform** (folded raised deck with holes, walls fold down, lock tabs)       | Perfume and premium bottles                  | Deck height; hole size taken at deck height; product alignment. Built (M1-A5) as one piece: two tube legs along the long edges (wall, foot, riser) lock into deck slots with tabs; leg and tab sizes follow from the board and deck height | MVP   |
| **Pocket tray**                                                                       | Kits with mixed items                        | Tray walls + internal cells                                                                                                                                                                | v1    |
| **Neck holder**                                                                       | Bottles with necks                           | Neck hole; legs height                                                                                                                                                                     | v1    |
| **Layered nest** (U15)                                                                | Premium, plastic-free "foam look"            | Layers count; outline per layer                                                                                                                                                            | v2    |

### 6.3 Outer box

- **Styles:** mailer box (FEFCO 0427 style, hinged lid with locking tabs), shipping box (FEFCO 0201, regular slotted container), and a paperboard tuck-end carton [16].
- **Sizing:**
    - Inside size = insert outer size + clearance on each side; height = the tallest product or insert + clearance.
    - Outside size follows from each style's layers of board (t = thickness, g = slot tolerance): mailer + 6t + 2g long, + 3t + g wide, + 2t tall (double end walls); shipping box + 2t, + 3t (joint flap), + 4t (two layers of flaps top and bottom).
    - Fine-tune the allowance for each material with real test cuts.
- **Show next to the box:**
    - Inside and outside sizes.
    - Board area (m²) and weight.
    - **Empty-space ratio** ≈ 1 − (product volume ÷ box inside volume). Label it "estimate": the EU calculation method is due by 12 Feb 2028 [17].

### 6.4 Materials and starting tolerances (verify with test cuts)

| Material                            | Typical thickness | Clearance per side (start) | Slot tolerance (start) |
| ----------------------------------- | ----------------- | -------------------------- | ---------------------- |
| Paperboard (folding boxboard / SBS) | ~0.4–0.6 mm       | 1.0 mm                     | +0.3 mm                |
| Chipboard / greyboard               | ~1–2.5 mm         | 1.0 mm                     | +0.3 mm                |
| Corrugated E-flute                  | ~1.2–2 mm         | 1.5 mm                     | +0.5 mm                |
| Corrugated B-flute                  | ~2.5–3.5 mm       | 1.5–2 mm                   | +0.5 mm                |
| Corrugated C-flute                  | ~3.5–4.5 mm       | 2 mm                       | +0.5 mm                |

These are common ranges and my starting values. Users can edit everything, and the app tells them to "check with your printer".

### 6.5 Manufacturability checks (warnings panel)

- Holes closer than **3 mm or 3× board thickness** to each other or to an edge.
- A hole within 2× thickness of a crease line.
- A slot width that doesn't match the board thickness (slot tolerance outside 0.1–1 mm, **estimate**).
- Board, or a cut-out, narrower than 1 mm.
- Crease lines crossing cut lines.
- Open cut paths, or duplicate lines (die makers dislike double lines).
- A piece bigger than the chosen sheet or Cricut mat.
- A deck so full of holes that it gets weak (cut-out area > 60% of the panel, **estimate**).

### 6.6 Exports by customer type

| Customer            | Files                                                                                                                             |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Printer / die maker | **PDF** (vector, separate CUT and CREASE layers; v2 adds named spot colors) and **DXF** (CUT/CREASE layers, mm). CF2 comes in v2. |
| Laser / CNC maker   | DXF/SVG with an optional kerf (cut-width) offset                                                                                  |
| Cricut / Silhouette | SVG with the score lines on their own layer, laid out on 12×12″ or 12×24″ mats                                                    |
| Anyone (free)       | Watermarked **print-at-home test PDF**, tiled across A4/Letter with overlap and alignment marks                                   |
| Marketing           | PNG renders (isometric), MP4/WebM unboxing video, GLB 3D model                                                                    |

---

## 7. 2D editor & 3D fold preview (React + Three.js)

### 7.1 Frontend stack

- **Core:** React + TypeScript + **Next.js** (App Router). Landing, SEO template pages and the app live in one codebase.
- **Styling:** vanilla CSS with CSS Modules and design tokens. **No Tailwind.**
- **State and API calls:** React built-ins only. `useReducer` for the editor (undo/redo is a reducer that keeps a history), Context for the session, and a small typed `fetch` hook for API calls. No Zustand, no TanStack Query.
- **3D:** `three` + `@react-three/fiber`. No drei: orbit controls and the orthographic camera come from three's own addons (`three/addons`).
- **Allowed runtime dependencies** (DEC-05 in [MILESTONES.md](MILESTONES.md)):
    - `next`, `react`, `react-dom`
    - `three`, `@react-three/fiber`
    - `exifr`
    - Paddle.js (from Paddle's CDN)
    - the `insert_core` WASM
    - the Plausible analytics script

    Anything else needs a written reason. Dev-only tools (types, lint, tests) are fine.
- **2D editor:** **SVG**, not WebGL. Lines stay crisp and selecting/dragging is easier. Cut lines solid, creases dashed, holes and dimensions shown.
    - Desktop gets full editing: drag products inside the insert, change cells, undo/redo.
    - Phone gets capture and view.
- **Photo upload:**
    - Read EXIF (focal length, orientation) in the browser with `exifr`.
    - Turn HEIC into JPEG through a canvas. This drops the EXIF, so send the values you read as separate fields.
    - Only Safari and other WebKit browsers (that includes every browser on iPhone) can draw HEIC on a canvas. Chrome and Firefox on desktop can't. For an HEIC file from those browsers, the MVP asks the user for a JPEG instead.
    - Resize to at most 4000 px.

### 7.2 3D fold algorithm (hinge tree)

1. Each piece is made of **panels** (polygons) joined by **hinges** (crease lines). The engine already knows these because it generates the shape (§6.1).
2. Pick a **root panel**, such as the deck or the box bottom. Walk the hinge graph outward from the root.
3. For each child panel: `child_transform = parent_transform × rotate(turn_line, fold_deg × t)`, where `t` goes from 0 to 1 with easing. The turn line is the crease, except for a flap that bends round another layer (a shipping box's outer flaps): there it lies `offset_mm` past the crease, and the strip before it stays with the parent. Fold in stages (walls first, tabs last) for a nicer animation.
4. Draw each panel as the polygon **extruded by the board thickness**, with kraft or white material.
5. **Partition grid:** animate the strips sliding down into each other's slots.
6. **Isometric camera:** orthographic, rotated 45° and tilted ≈ 35.26°. Also offer orbit view, an "exploded" view, and a fold slider from 0% to 100%.

### 7.3 Fit check and unboxing video

- **Product stand-ins in 3D:**
    - The outline is extruded to the product's height, giving a prism.
    - In v1, the side-photo profile is turned around its axis to make a bottle shape (Three.js lathe).
- **Fit colors:** measure each product's gap to what surrounds it, seen from above (its hole's edge, grid strips, cradle legs, the insert's edge).
    - Green: clearance ≥ target, the clearance recommended for the insert's board (§6.4).
    - Yellow: tight.
    - Red: collides (board, another product, a leg, or off the insert).
- **Unboxing video:**
    - Record the canvas (`canvas.captureStream()` + `MediaRecorder`) as WebM or MP4, depending on the browser.
    - Add a watermark on the free plan.
    - Optionally convert to MP4 or GIF on the server.

### 7.4 Protecting paid files

- **Production vectors are made only on the server**, after the entitlement check (§9.4).
- The browser preview gets plain straight segments within 0.05 mm of the true lines, points to 0.01 mm, no arcs and no layers. It draws smooth when zoomed but isn't a production file. (0.5 mm rounding was tried first and made curves stair-stepped: M1-A10.)
- Free PNG and PDF exports carry a watermark. The print-at-home test PDF draws the preview's lines, with no layers, and each tiled page holds only its own part of a piece (M1-B4). The PNG preview draws the same preview geometry as pixels (M1-B5).
- Download links are short-lived signed URLs.

Someone determined could still copy a preview. That's acceptable: most buyers want clean, layered production files.

---

## 8. System architecture

### 8.1 Diagram

```text
Phone / Desktop browser
  React + R3F (Three.js) + insert_core.wasm (live preview)
        │ HTTPS: JSON + multipart upload                 Paddle.js overlay checkout
        ▼                                                         │
┌──────────────────────── Axum API (Rust) ────────────────────────┐│
│ auth · designs · captures · exports · share · billing           ││
│                                                                  ││
│  Vision worker ── L1 OpenCV ─▶ L2 SAM 2 (Pure Rust Candle) ─▶ L3 AI     │◀── HTTPS, only when unsure: HF → Gemini → fal.ai
│  Export worker ── insert_core (native) ─▶ SVG / PDF / DXF       ││
│  Paddle webhook ─▶ entitlements (unlock / credits / subscription)│◀── Paddle (signed webhooks)
└───────┬──────────────────────────────┬──────────────────────────┘│
        ▼                              ▼                           ▼
   PostgreSQL                  Object storage (Cloudflare R2)    Paddle (MoR: tax,
   users, designs,             photos (auto-delete),             fraud, refunds,
   entitlements, events        exports (signed URLs)             payouts to you)
```

### 8.2 Components

| Part          | Choice                                                                                | Notes                                                     |
| ------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Web framework | Axum + Tokio + tower-http (CORS, compression, body limit, tracing)                    | Use the latest stable versions                            |
| Database      | PostgreSQL + `sqlx`                                                                   | A small managed Postgres or one on the VPS                |
| Jobs          | A Postgres job table (`FOR UPDATE SKIP LOCKED`) or a Rust queue crate                 | Vision and export run as jobs, not inside the web request |
| Vision        | `opencv` crate (needs system OpenCV + clang), `Candle`                          | Build in Docker with OpenCV runtime libraries             |
| Files         | S3-compatible storage (e.g. Cloudflare R2)                                            | Signed URLs; delete photos automatically                  |
| Auth          | Email magic link (your own) **or** a hosted auth service                              | Hosted auth saves 1–2 weeks **(estimate)**                |
| Emails        | A transactional email API                                                             | Magic links and receipts. Paddle also emails receipts.    |
| Monitoring    | `tracing` logs + error tracking (e.g. Sentry) + product analytics (Plausible script) | Track the funnel in §13                                   |
| Hosting       | 1 VPS (4 vCPU / 8 GB) + CDN; frontend on static hosting                               | Add a GPU only if SAM 2 on CPU is too slow                |

### 8.3 API endpoints (v1)

```text
POST   /api/v1/captures                 multipart image + {paper, mode, exif}  → {capture_id}
GET    /api/v1/captures/{id}            → {status, paper, products[], warnings[], level_used}
POST   /api/v1/captures/{id}/correct    {tap_point | edited_contour}           → re-run measure
POST   /api/v1/designs                  {products, insert, material, box}      → {design_id}
GET    /api/v1/designs/{id}   PATCH /api/v1/designs/{id}
POST   /api/v1/designs/{id}/exports     {formats, preset}  → 200 {export_id} | 402 {checkout}
GET    /api/v1/exports/{id}             → {status, files:[{format, signed_url}]}
POST   /api/v1/paddle/webhook           (raw body, signature checked)
GET    /api/v1/me                       → {plan, credits, unlocked_designs, subscription}
POST   /api/v1/billing/portal           → Paddle customer portal link
POST   /api/v1/handoff                  → {token, qr_url}     GET /api/v1/handoff/{token}/events (SSE)
GET    /s/{slug}                        public view-only 3D share page
```

### 8.4 Database tables

```text
users(id, email, locale, paper_default, created_at)
captures(id, user_id, storage_key, exif jsonb, mode, paper, result jsonb, confidence, level_used, created_at, purge_after)
products(id, user_id, capture_id, name, contour_mm jsonb, bbox_mm jsonb, shape, height_mm, profile jsonb)
designs(id, user_id, spec jsonb, version, share_slug, created_at, updated_at)
design_unlocks(design_id, user_id, source[purchase|credit|pro], unlocked_at, edits_until)
credit_ledger(id, user_id, delta, reason, paddle_txn_id, created_at)      -- balance = SUM(delta)
subscriptions(id, user_id, paddle_subscription_id, plan, status, current_period_end, updated_at)
paddle_events(event_id PK, type, received_at, processed_at, payload jsonb) -- idempotency
exports(id, design_id, design_version, formats, status, files jsonb, created_at)
```

### 8.5 Security and privacy

- **Uploads:** 15 MB maximum. Check the file type and cap the pixel count to block "decompression bomb" images. Rate-limit uploads per user and per IP.
- **Photos:** remove GPS data from EXIF, delete photos after 30 days, and keep only outlines and sizes. Let users delete their data.
- **Paid files:** sent only through signed URLs that expire in minutes.
- **Code:** no user-supplied code ever runs on the server.

### 8.6 Monthly running cost at the start (estimate)

| Item                                               | $/month (estimate) |
| -------------------------------------------------- | ------------------ |
| VPS (4 vCPU / 8 GB)                                | 20–40              |
| PostgreSQL                                         | 0–25               |
| Object storage + CDN                               | 0–5                |
| Email, domain, error tracking (free tiers)         | 1–10               |
| Plausible analytics (paid plan, or self-host free) | 0–20               |
| Backup AI (§5.4): HF credits + fal.ai; Gemini free tier is $0 | ~1–5    |
| **Total**                                          | **≈ 25–105**       |

---

## 9. Payments with Paddle

### 9.1 Facts and checks

- **Fee:** **5% + 50¢ per transaction**, with no monthly fee [11].
- **Under $10:** Paddle says products under $10 should **contact them for custom pricing** [11]. **Keep every checkout at $10 or more.** The $10 single export sits exactly on this line, so never discount it.
- **Country:** Paddle works with software businesses anywhere except a list of unsupported countries. **Indonesia is not on that list**, but the page doesn't describe payout methods [12]. Ask Paddle support how payouts to an Indonesian bank work, and in which currency, **before building billing code**.
- **Domain approval checklist** [13]:
    - Live HTTPS site.
    - Clear product description and key features.
    - Pricing page.
    - **Terms & Conditions with your legal name** (sole proprietor).
    - Refund Policy and Privacy Policy in the site menu.
    - Your terms say that Paddle is the merchant of record.
- **Slow verification:** some reviewers report slow identity checks and surprise requirements [15]. **Apply in Week 1** with the landing page live.
- **Why not Stripe directly:** Stripe accounts based in Indonesia can only accept rupiah through virtual accounts [25]. A merchant of record solves foreign cards and tax.

### 9.2 Catalog (set up in sandbox first)

| Paddle product          | Price                  | Type                 |
| ----------------------- | ---------------------- | -------------------- |
| Export – single design  | $10                    | one-time             |
| Export pack – 5 designs | $35                    | one-time (5 credits) |
| Pro                     | $12/month or $139/year | subscription         |
| Studio (3 seats)        | $49/month              | subscription (v2)    |

### 9.3 Checkout and unlock flow

```text
User clicks Export
  └─▶ POST /designs/{id}/exports
        ├─ Pro active?            → create export job → download
        ├─ Credit balance > 0?    → spend 1 credit (ledger −1) → unlock design → job
        └─ else                   → 402 {priceId, customData:{user_id, design_id}}
              └─▶ Paddle.Checkout.open({items:[{priceId,quantity:1}], customer:{email}, customData})
                    └─▶ customer pays (Paddle handles card, tax, receipt)
                          └─▶ webhook transaction.completed → verify → unlock design
                                └─▶ browser gets "unlocked" (SSE or polling /me) → export job → download
```

### 9.4 Webhooks and entitlements

- **Verify the signature:**
    - Header `Paddle-Signature: ts=…;h1=…`.
    - Compute HMAC-SHA256 over `ts + ":" + raw_body` with the endpoint secret, and compare in constant time [14].
    - Use the **raw body**: parsing the JSON first breaks verification.
    - Reject timestamps older than a few minutes (your choice).
- **Idempotency:** store `event_id` in `paddle_events` (primary key). Skip events you've already processed. Return 200 quickly and do the work in a job.
- **Events to handle:**
    - `transaction.completed`: unlock a design or add credits, based on `custom_data`.
    - `subscription.created` / `updated` / `canceled` / `past_due`: update `subscriptions`.
    - Refund and chargeback adjustments: remove credits or the unlock.
- **Daily repair job:** fetch active subscriptions from the Paddle API and fix any drift.
- **Entitlement rules:**
    - An unlocked design can be re-downloaded forever and **edited for 30 days**.
    - Credits never expire.
    - Pro means unlimited exports while the subscription is active.
- **Keep billing behind one Rust trait** (`BillingProvider`) so you can switch to another merchant of record later (§9.6).

### 9.5 Fee math (before currency conversion and payout fees, which are unknown)

| Sale          | Paddle fee | You get |
| ------------- | ---------- | ------- |
| $10 export    | $1.00      | $9.00   |
| $35 pack      | $2.25      | $32.75  |
| $12 Pro month | $1.10      | $10.90  |
| $139 Pro year | $7.45      | $131.55 |
| $49 Studio    | $2.95      | $46.05  |

### 9.6 Plan B merchants of record

If Paddle rejects you or can't pay out to Indonesia, the alternatives are:

- **Lemon Squeezy**: a 2026 roundup says it pays out through PayPal to 200+ countries, which likely covers Indonesia.
- **Polar**, **Dodo Payments** or **Creem**.

Confirm each one's Indonesia payouts first [26].

---

## 10. Pricing

| Plan              | Price            | Includes                                                                                                          |
| ----------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Free**          | $0               | Unlimited capture, design and 3D preview; 1 product type per design; watermarked PNG + **print-at-home test PDF** |
| **Single export** | $10 / design     | SVG + PDF + DXF, spec sheet, 3D renders; re-download any time; edits for 30 days                                  |
| **5-pack**        | $35              | 5 export credits, never expire                                                                                    |
| **Pro**           | $12/mo · $139/yr | Unlimited exports, kit builder, maker presets, unboxing video without watermark, commercial use                   |
| **Studio** (v2)   | $49/mo           | Pro + 3 seats, CF2, spot-color PDF, brand library, priority support                                               |

Why these numbers:

- Pacdora's paid plans are listed at about $12–29/month, and its Pro (~$29) has unlimited dieline exports [6]. Pro at $12 costs less than half of that and adds the photo→fit features.
- Single exports sit exactly on Paddle's $10 line. No coupon or discount may take them below $10.
- Pro costs about the same as one export, so expect most buyers to choose Pro. Watch the single-export vs. Pro mix in the first 3 months.
- Studio at $49 costs more than 3 separate Pro plans (3 × $12 = $36). The extra price is for the print-shop features (CF2, spot-color PDF, brand library, priority support), so it is not a seat discount. Check this again before v2.
- Test prices for the first 3 months, then adjust.

Example (not a forecast): 60 Pro subscribers + 80 single exports a month ≈ 60 × $10.90 + 80 × $9.00 ≈ **$1,374/month** after Paddle fees.

---

## 11. Getting customers without cold sales

1. **Template library for search (SEO).** Publish 100+ pages such as:
    - "30 ml perfume bottle box insert template"
    - "8 oz candle jar insert"
    - "6-bottle partition divider generator"
    - "lip gloss tube insert"
    - "egg-crate divider template"

    Each page has a live 3D preview, a free test PDF and a "make it fit **your** product → snap a photo" button. Avoid brand names in titles.

2. **How-to guides:**
    - How to make a box insert.
    - Insert vs. partition.
    - Choosing E-flute or B-flute.
    - Getting inserts made by a printer.
3. **Short videos** (YouTube Shorts, TikTok, Reels): "photo → insert in 60 s", before and after, unboxing renders, and "I made this on a Cricut".
4. **Communities.** Answer real questions and share free templates, never spam:
    - Reddit: r/packaging, r/smallbusiness, r/Etsy, r/cricut, r/lasercutting.
    - Candle, soap, perfume and cosmetics maker groups.
5. **Things users share:** view-only 3D share links and watermarked unboxing videos. Both bring new users for free.
6. **Launch sites:** Product Hunt, AI tool directories, and Show HN (about the Rust/WASM/vision engineering).
7. **Later:** a "for printers" page offering the embeddable widget (U14). Printers come to you.

---

## 12. Roadmap (solo, week by week)

| Phase                | Weeks    | Deliverables                                                                                                                                                                                                                                                                                                                | Exit check                                                                             |
| -------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **0. Validate**      | 1–2      | Landing page with a demo video. Waitlist. A "Download files – $10" **fake-door button** that leads to "join the beta". 5 short videos. **Accuracy spike**: OpenCV paper + outline on 10 objects (§5.6). **Apply to Paddle.** Make 10–20 designs for beta users by hand, for free.                                           | ≥ 100 signups or ≥ 10% fake-door clicks, **and** accuracy on track **(suggested bar)** |
| **1. MVP build**     | 3–8      | `insert_core` (partition grid, cutout card, cradle; mailer + shipping box) with unit tests. Vision L1 + Trace mode + manual fixes. React capture flow, SVG editor, R3F fold preview, 2D fit check. SVG/PDF/DXF export + print-at-home PDF. Paddle sandbox → live: checkout, webhooks, credits, Pro. Legal pages. Analytics. | End-to-end on 3 phones; 0 open payment bugs                                            |
| **2. Beta & launch** | 9–10     | 30 beta users. Fix the top 10 issues. 30 SEO template pages. Launch on Product Hunt and in communities.                                                                                                                                                                                                                     | First paying customers                                                                 |
| **3. v1**            | 11–16    | Kit builder (U5). L2 SAM 2 + L3 AI helper (HF → Gemini → fal.ai). Lay-down photo + pose correction (U7). Live camera guide + level (U6). QR handoff. Maker presets (U8). Unboxing video (U9). Spec sheet (U10). Empty-space ratio (U11). AI advisor (U12).                                                                                     | Conversion and accuracy metrics in §13                                                 |
| **4. v2**            | month 5+ | CF2 + spot-color PDF (U13), Studio seats, pocket tray / neck holder / layered nest, more languages (Bahasa Indonesia, Spanish, Portuguese…), printer widget (U14)                                                                                                                                                           | Studio demand from inbound requests                                                    |

---

## 13. Metrics and go/no-go rules

| Metric                                             | Healthy target (suggested)                                                                            |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Visitors who upload a photo or type sizes          | ≥ 15%                                                                                                 |
| Detections that need no manual fix                 | ≥ 80%                                                                                                 |
| Started designs that are finished                  | ≥ 50%                                                                                                 |
| Finished designs that are paid for (export or Pro) | ≥ 3–5%                                                                                                |
| Revenue 3 months after launch                      | ≥ $1,000/month → keep going · < $300/month → change focus (e.g. kits for gift brands, or print shops) |

---

## 14. Risks and mitigations

| Risk                                                     | Impact            | Mitigation                                                                                                                                        |
| -------------------------------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wrong sizes on tall or shiny products                    | Bad fits, refunds | Trace mode, lay-down + height, pose correction, a confidence score, manual fixes, a mandatory "print a test first" step, clear clearance defaults |
| Glass, transparent or white products are hard to cut out | Detection fails   | Colored paper, side light, L2 SAM 2, L3 hints, Trace mode                                                                                         |
| Paddle rejects you or can't pay out to Indonesia         | No revenue        | Apply in Week 1; billing trait; Plan B merchants of record (§9.6)                                                                                 |
| Pacdora (or others) add photo → insert                   | Less unique       | Ship fast; own the niche with kits, makers, Trace mode and fit check; SEO template moat; better accuracy                                          |
| Paid files leaked from the preview                       | Lost sales        | Server-only production files, rounded previews, watermarks, signed URLs                                                                           |
| OpenCV + Rust build pain                                 | Slow progress     | Docker build image from day 1; keep the vision code small; pure-Rust fallbacks (`imageproc`) for simple steps                                     |
| Model license mistakes                                   | Legal risk        | SAM 2 (Apache 2.0); check BiRefNet weights; **no RMBG-2.0**; read SAM 3's license                                                                 |
| Makers pay little                                        | Low revenue       | Focus paid features on brands and designers; makers bring traffic and shares                                                                      |
| Scope creep (packaging CAD is deep)                      | Never launch      | MVP = 3 insert styles + 2 box styles + 3 export formats. Nothing else.                                                                            |
| Product liability ("my bottle broke")                    | Disputes          | Terms: files are design aids; always test-print; no shipping-protection claims                                                                    |

---

## 15. Decisions for you

1. **Name and domain.** "paccradle" is only a working name.
2. **Auth:** your own magic links in Axum, or a hosted auth service, which saves time.
3. **First 3 insert styles:** partition grid, cutout card and cradle are my suggestion.
4. **Height capture in the MVP:** type it, or the lay-down photo.
5. **Backup AI model:** decided 2026-10-28. Hugging Face (`Qwen3-VL-8B-Instruct`) first, then the Gemini free tier (skipped for visitors in the EEA, the UK and Switzerland), then fal.ai (Moondream 3). All three are vision-language models, not image generators. No Claude API. See §5.4 and DEC-13 in [MILESTONES.md](MILESTONES.md).
6. **Prices:** decided 2026-10-27: $10 export / $35 5-pack / $12 Pro per month / $139 Pro per year; Studio $49 (v2). See DEC-02 in [MILESTONES.md](MILESTONES.md).
7. **Languages at launch:** English only, or English + Bahasa Indonesia.

---

## 16. Sources

1. CEF Box insert generator — https://www.cefbox.com/dielines/insert/custom-insert-23022
2. PackagingDieline.com — https://packagingdieline.com/
3. DielineMaker — https://dielinemaker.com/tools/dieline-generator
4. Pacdora dieline generator — https://www.pacdora.com/tools/dieline-generator
5. Pacdora AI Creation launch (Apr 2026) — https://technode.global/prnasia/pacdora-redefines-packaging-design-with-ai-driven-innovation/
6. Pacdora pricing (third-party listings) — https://www.grabon.in/pacdora-coupons/ · https://www.ki-syndikat.de/tools/pacdora/
7. toolKaiser (photo → foam insert DXF/SVG) — https://apps.apple.com/us/app/-/id1528414619
8. TooltraceAI review, CNC Kitchen — https://www.cnckitchen.com/blog/this-free-ai-tool-auto-generates-gridfinity-bins
9. Photo-to-DXF methods (Shadow Foam guide; V1 Engineering forum) — https://www.shadowfoam.com/making-your-own-custom-foam-inserts/ · https://forum.v1engineering.com/t/best-method-for-kaizen-inserts/29723
10. QinPrinting box templates — https://www.qinprinting.com/box-templates/
11. Paddle pricing — https://www.paddle.com/pricing
12. Paddle supported countries — https://www.paddle.com/help/start/intro-to-paddle/which-countries-are-supported-by-paddle
13. Paddle domain approval — https://www.paddle.com/help/start/account-verification/what-is-domain-approval
14. Paddle webhook signatures (Hookdeck guide) — https://hookdeck.com/webhooks/platforms/how-to-secure-and-verify-paddle-webhooks-with-hookdeck
15. Paddle user reviews (anecdotal) — https://www.capterra.ca/reviews/173382/paddle
16. FEFCO codes — https://www.fefco.org/sites/default/files/documents/FEFCO_Codes_of_Designs.pdf · 0934 — https://www.vksshop.com/en/Packaging-designs/Interior-fitments-FEFCO-09XX/Cut-out-FEFCO-0934.html · 0427 — https://kilbypackaging.madeinthemidlands.com/news/what-is-an-0427-box · 0201 — https://lilpackaging.com/pages/0201-boxes
17. EU PPWR empty-space rules — https://www.fieldfisher.com/en/insights/new-eu-packaging-and-packaging-waste-rules-10-key-things-every-global-business-should-know · https://www.ship24.com/blog/eu-packaging-rules-are-live-what-ppwr-means-for-ecommerce-parcels
18. SAM 2 (Apache 2.0 checkpoints, model sizes) — https://github.com/facebookresearch/sam2
19. SAM 3 license — https://huggingface.co/facebook/sam3/blob/main/LICENSE
20. BiRefNet (MIT repo, ONNX, lite variant) — https://github.com/ZhengPeng7/BiRefNet
21. RMBG-2.0 (CC BY-NC 4.0) — https://github.com/Bria-AI/RMBG-2.0
22. Rust `opencv` crate 0.101.1 — https://docs.rs/crate/opencv/latest
23. Rust `ort` crate 2.0.0-rc.13 — https://docs.rs/crate/ort/2.0.0-rc.13/source/README.md
24. Hugging Face Inference Providers pricing (free accounts: no monthly credits; checked 2026-10-28) — https://huggingface.co/docs/inference-providers/pricing
25. Stripe accounts in Indonesia — https://support.stripe.com/questions/supported-payment-methods-currencies-and-businesses-for-stripe-accounts-in-indonesia
26. Merchant-of-record comparison 2026 — https://www.shareuhack.com/en/posts/digital-product-platform-comparison-asia-2026
27. Gemini API terms (unpaid services, EEA/UK/Switzerland rule; checked 2026-10-28) — https://ai.google.dev/gemini-api/terms · object detection with `box_2d` — https://ai.google.dev/gemini-api/docs/image-understanding
28. fal.ai Moondream 3 detect (input, output and latency checked 2026-10-28) — https://fal.ai/models/fal-ai/moondream3-preview/detect
