# Paccradle — Milestones & Execution Tracker

> This file turns the plan in [CLAUDE.md](CLAUDE.md) into tasks we do one by one.
> CLAUDE.md says **what and why**. This file says **what's next and whether it's done**.
> References like §5.3 point to sections of CLAUDE.md.

**Created:** 2026-10-27 · **Week 1 starts:** Mon 26 Oct 2026

---

## How to use this file

- **One task at a time.** Work top to bottom inside the current milestone. Each task is sized to fit in about one working session.
- **Owner** on each task:
    - _you_: needs you (accounts, money, photos, posting, legal name, real phones).
    - _claude_: code or docs I can do in this repo.
    - _both_: I prepare, you do the part only you can do.
- **When a task is done:** tick it `[x]`, add `(done YYYY-MM-DD)`, add one line to the [Progress log](#progress-log), and update [Current focus](#current-focus).
- **Decisions** go in the [Decision log](#decision-log). If a decision changes the plan, CLAUDE.md is updated too.
- **Gates:** don't start the next milestone until its gate passes. If you skip a gate, write down why in the Decision log.
- **New ideas mid-build** go to the [Parking lot](#parking-lot), not into the current milestone (§14 scope creep).
- **To resume in any session, say:** "next task", "continue M1-A", or "do M0-04".
- if you want to generate real pic you can use mcp openrouter image generator i have here, maybe we can use cheap model image generator to create realistic test image that we want to do vision for our goal

---

## Current focus

| Item              | Value                                                                       |
| ----------------- | --------------------------------------------------------------------------- |
| Milestone         | **M0: Validate & set up** (your tasks) · **M1: MVP build** started early (see Decision log, 2026-10-28) |
| Next task         | **M1-D4**: uploads & storage (C2's remaining checks and C5 wait for the M0-07 photos) · **M1-B3**: your check of the DXF in LibreCAD or Inkscape and your cutter app · **M0-07**: test photos (yours), then I run the M0-11 report |
| Waiting on        | You: B3 check in your apps, a printed B4 test print (once), DEC-01, DEC-06, DEC-08 (confirm the 3 styles), DEC-09 (confirm "type it"), DEC-14 (sign-in at the first export?), M0-05 settings, M0-06 Paddle, M0-07 photos |
| Blockers          | None for M1-A. M0-03: first commit exists (`0284845`); the clean-clone Docker check is still to run |
| Last updated      | 2026-10-29                                                                  |

---

## Where things stand (27 Oct 2026)

| Area                       | State                                                                                                                                                                                                                       |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Plan                       | [CLAUDE.md](CLAUDE.md), status "before validation"                                                                                                                                                                          |
| `pac-cradle/` (this folder) | Monorepo root (DEC-03): `git init` done, **no commits yet**. CI workflow in `.github/workflows/ci.yml` (runs once the repo is on GitHub).                                                                                    |
| `paccradle-backend/`       | Cargo workspace: the API in `src/` (client and admin surfaces, middleware, probes: M1-D1; Postgres schema, job queue and worker: D2; sign-in with email links, passwords, passkeys and the back office's audit log: D3), `insert_core` (data model, geometry helpers, the 3 insert styles, 2 outer boxes, the manufacturability checks, the 2D fit check and golden drawings: M1-A1 to A9), `insert_core_wasm` (the browser build, A10), and the `vision` spike (M0-08 to M0-10). `insert_core` needs no OpenCV, so plain `cargo test -p insert_core` works on this Mac. Docker toolchain image with OpenCV 4.10. Builds need OpenCV, so on this Mac run cargo through `scripts/in-docker.sh`. |
| `paccradle-frontend-app/`  | Next.js 16, no Tailwind (CSS Modules + tokens). Landing, pricing and legal pages built (M0-04) and redesigned 2026-10-28: eco-packaging palette, Fraunces type, animated dieline hero, scroll and hover motion. Not deployed. |
| Repo rules (`.claude/rules`) | Rewritten for Paccradle (M0-02).                                                                                                                                                                                          |
| Domain, Paddle, photos     | Not started (yours: DEC-01, DEC-06, M0-05, M0-06, M0-07)                                                                                                                                                                   |
| This Mac                   | Node on the PATH is v12; the frontend needs `nvm use` (v24). OpenCV isn't installed (the Homebrew build from source was stopped).                                                                                         |

---

## Timeline

| Milestone                  | Weeks    | Dates                      | Gate   |
| -------------------------- | -------- | -------------------------- | ------ |
| **M0** Validate & set up    | 1–2      | 26 Oct – 8 Nov 2026        | Gate 0 |
| **M1** MVP build            | 3–8      | 9 Nov – 20 Dec 2026        | Gate 1 |
| **M2** Beta & launch        | 9–10     | 21 Dec 2026 – 3 Jan 2027   | Gate 2 |
| **M3** v1                   | 11–16    | 4 Jan – 14 Feb 2027        | Gate 3 |
| **M4** v2                   | month 5+ | from ≈ Mar 2027            | Gate 4 |

> ⚠️ M2 falls on **Christmas and New Year**. Product Hunt and community launches do badly that week. See DEC-10.

---

## Open decisions

Ordered by when they're needed. §15 of the plan lists 7. The rest come from conflicts I found between the plan and the repos.

| ID     | Decision                                       | Options                                                                                       | My suggestion                                                                                                                                                           | Needed by | Status |
| ------ | ---------------------------------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------ |
| DEC-01 | Name & domain (§15.1)                          | keep "paccradle" · rename                                                                     | Decide before the landing page. Check that the .com and social handles are free.                                                                                        | M0-04     | open   |
| DEC-02 | Prices (§15.6)                                 | $12 / $45 / $19 / $159 · $10 / $35 / $12 / $139                                              | **$10 / $35 / $12 / $139, Studio $49.** See below.                                                                                                                       | M0-04     | **decided 2026-10-27** |
| DEC-03 | Repo layout                                    | one monorepo at `pac-cradle/` · 2 separate repos                                              | **Monorepo, crates inside the backend.** See the layout below.                                                                                                           | M0-03     | **decided 2026-10-27** |
| DEC-04 | Frontend framework                             | Next.js (already scaffolded) · Vite SPA                                                       | **Next.js.** Landing, SEO pages (§11.1) and the app live in one codebase.                                                                                               | M0-02     | **decided 2026-10-27: Next.js** |
| DEC-05 | Frontend dependencies & styling                | your rules: React only · plan: Zustand, TanStack Query, R3F, drei                             | **No Tailwind** (vanilla CSS, CSS Modules + design tokens). **Runtime dependencies allowed:** `next`, `react`, `react-dom`, `three`, `@react-three/fiber` (no drei), `exifr`, Paddle.js, the `insert_core` WASM and the Plausible script. State uses React built-ins only. | M0-02     | **decided 2026-10-27** |
| DEC-06 | Hosting (§8.2)                                 | VPS provider · managed or self-hosted Postgres · frontend host                                |  (your call on provider)                                                                                                                                                | M0-05     | open   |
| DEC-07 | Auth (§15.2)                                   | own magic links · hosted auth service                                                         | **Own sign-in** (your D3 notes): an email link confirms the address, then a password, then a passkey where the device supports one; passkey first at sign-in, password after 2 failed tries, an email link for a forgotten password. The back office: passkey only. JWT + rotating refresh cookie. | M1-D3     | **decided 2026-10-29**, built in M1-D3 |
| DEC-08 | First 3 insert styles (§15.3)                  | —                                                                                             | Partition grid, cutout card, cradle platform                                                                                                                            | M1-A      | open   |
| DEC-09 | Height capture in MVP (§15.4)                  | type it · lay-down photo                                                                      | **Type it** (fix B, §5.2). Lay-down photo + pose correction come in M3.                                                                                                  | M1-C4     | open: built as suggested in M1-C4, confirm |
| DEC-10 | Launch timing                                  | public launch in weeks 9–10 (holidays) · move it                                              | **Private beta over the holidays, public launch in the week of 11 Jan 2027.** M3 shifts by about a week.                                                                 | M2        | open   |
| DEC-11 | Languages at launch (§15.7)                    | EN · EN + Bahasa Indonesia                                                                    | **EN at launch.** All strings go through `t()` from day 1 (your rule), so adding ID later is cheap.                                                                     | M2        | open   |
| DEC-12 | Pocket tray / neck holder phase                | §6.2 says v1 · §12 says v2                                                                    | **v2**, following the roadmap (scope risk, §14)                                                                                                                          | M3        | open   |
| DEC-13 | Backup AI model (§15.5)                        | Image generators (SD line art, ControlNet) can't find products · vision-language models       | **Hugging Face `Qwen3-VL-8B-Instruct` → Gemini free tier (not for EEA/UK/Swiss visitors) → fal.ai Moondream 3.** No Claude API. See §5.4. Run all three on the M0 photo set in M3-02. | M3-02     | **decided 2026-10-28** |
| DEC-14 | When sign-in is asked for                      | before the first capture · at the first export · only at checkout                              | **At the first export, the free test print included.** Visitors capture, design and preview as guests (the §13 upload target); the free test print (U3) becomes the sign-up step, and Paddle needs an account anyway. D3 works either way; D7 enforces it. | M1-D7     | open: suggested in M1-D3, confirm |

### DEC-02: decided prices (2026-10-27)

| Product         | Price      | Paddle fee (5% + 50¢) | You get  |
| --------------- | ---------- | --------------------- | -------- |
| Single export   | $10        | $1.00                 | $9.00    |
| 5-pack          | $35        | $2.25                 | $32.75   |
| Pro, monthly    | $12        | $1.10                 | $10.90   |
| Pro, yearly     | $139       | $7.45                 | $131.55  |
| Studio (v2)     | $49/month  | $2.95                 | $46.05   |

- CLAUDE.md now uses these prices everywhere: §0, the §3 flow, §9.1, §9.2, §9.5, §10, §12 and §15.6.
- **The $10 export sits exactly on Paddle's $10 line** (§9.1). Never put a coupon or discount on it.
- **Things to watch:**
    - Pro ($12) costs about the same as one export, so most buyers will likely pick Pro. Watch the mix in the first 3 months.
    - Studio ($49) costs more than 3 Pro plans ($36), so it has to win on its print-shop features. Check it again before v2.
    - The §13 rule (≥ $1,000/month after 3 months) now needs about **92 Pro subscribers, or about 111 single exports a month**. Under the old $12/$19 plan it took about 57 Pro subscribers.

### DEC-03: decided repo layout (2026-10-27)

```text
pac-cradle/                      ← one git repo
├── CLAUDE.md                    plan
├── MILESTONES.md                this file
├── paccradle-backend/           Cargo workspace
│   ├── src/                     Axum API (your clean-architecture layout)
│   ├── crates/insert_core/      geometry: native + WASM (§6.1)
│   ├── crates/vision/           OpenCV pipeline + spike CLI (§5)
│   ├── migrations/              sqlx
│   └── postman/                 dated API contract files (your rule)
└── paccradle-frontend-app/      Next.js: landing + SEO pages + app
```

- **Why:** the plan and the code live in one place, there's one CI, and `insert_core` is shared without publishing a package.
- **All Rust code lives in one Cargo workspace** in `paccradle-backend/`. The Next.js app imports the WASM build of `insert_core` by local path.
- **The backend's own `.git` has no commits.** M0-03 removes it (after checking it's still empty) and runs `git init` at `pac-cradle/`.

---

## M0: Validate & set up (weeks 1–2)

**Goal:** prove that people want this and that the measuring works, before building the product (§12, phase 0).

### Set-up

- [ ] **M0-01 · Lock the set-up decisions** · _you_ · §15
    - Decide DEC-01 and DEC-06. DEC-02 to DEC-05 are already decided.
    - Done when: rows are filled in the Decision log, and I've updated CLAUDE.md where the plan changes.
- [x] **M0-02 · Rewrite the copied repo rules** · _claude_ (done 2026-10-27)
    - Backend `.claude/rules`: replace RepVane content with Paccradle.
        - Domains: identity, captures, designs, exports, billing, share.
        - Tables from §8.4.
        - Crates: `insert_core` and `vision`.
        - Remove trainers, meal photos, WhatsApp, health data, Xendit.
    - Frontend `.claude/rules`:
        - Next.js (App Router) instead of Vite and the custom History-API router.
        - Vanilla CSS (CSS Modules + design tokens), **no Tailwind**.
        - Allowed dependencies per DEC-05.
        - Remove PWA offline sync, charts for weight/macros, RepVane branding and the 8-language list.
    - Fix links to the missing `app-context.md`.
    - Pick one API-contract source. Today the rules mention both utoipa/OpenAPI and Postman.
    - Done when: no rule mentions another product, and the rules agree with CLAUDE.md.
- [ ] **M0-03 · Repo set-up** · _claude_ · DEC-03, §14
    - Monorepo per DEC-03:
        - Check that `paccradle-backend/.git` still has no commits, then remove it.
        - `git init` at `pac-cradle/`, with a root `.gitignore` covering `target/`, `node_modules/`, `.next/`, `.env*` and `.DS_Store`.
    - Turn `paccradle-backend/` into a Cargo workspace. The API stays in `src/`; add empty `crates/insert_core` and `crates/vision`.
    - **Docker build image with OpenCV + clang from day 1** (§14, §5.3).
    - Frontend clean-up:
        - Remove Tailwind from `paccradle-frontend-app` (the `tailwindcss` and `@tailwindcss/*` packages, and the import in `globals.css`).
        - Add `styles/tokens.css` for design tokens and use CSS Modules.
    - CI (GitHub Actions):
        - Backend: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `cargo-audit`.
        - Frontend: `tsc --noEmit`, lint, build.
    - Done when: a clean clone builds in Docker, and the first commit is made.
    - Status 2026-10-27: all done and checked (release image builds and runs; fmt, clippy and tests pass in Docker; frontend lint and build pass) **except the first commit**. CI only runs once the repo is pushed to GitHub.
- [x] **M0-04 · Landing page** · _claude_ · §9.1, §11 (done 2026-10-27; real values and deploy are in M0-05)
    - Sections:
        - Hero with a demo-video slot.
        - "How it works" in 3 steps.
        - Who it's for (§2).
        - Pricing page (DEC-02).
        - Waitlist form. Simplest option: a hosted email-list form, so week 1 needs no backend.
    - **Fake door:** a "Download files – $10" button that opens "Join the beta".
    - **Legal pages in the menu:**
        - Terms with your legal name, stating that Paddle is the merchant of record and that the files are design aids (§14).
        - Refund Policy.
        - Privacy Policy, including the photo and AI-provider notice (§5.4).
    - Plausible analytics (one script tag, DEC-05) with custom events `fake_door_click` and `waitlist_signup`. Plausible counts page views automatically.
    - Done when: the page passes the Paddle domain checklist (§9.1) line by line.
- [ ] **M0-05 · Domain + deploy the landing page** · _both_ · DEC-01, DEC-06
    - Fill in `paccradle-frontend-app/.env.local` (copy `.env.example`):
        - `NEXT_PUBLIC_LEGAL_NAME` and `NEXT_PUBLIC_CONTACT_EMAIL`. Until then the legal pages show visible placeholders.
        - `NEXT_PUBLIC_WAITLIST_ENDPOINT`: pick an email-list tool whose form endpoint accepts a POST with an `email` field.
        - `NEXT_PUBLIC_PLAUSIBLE_DOMAIN` (Plausible's hosted plan is paid; self-hosting is free).
    - Have the draft legal text in `content/en/legal.ts` reviewed. The 14-day refund window is my draft choice.
    - Done when: the page is live over HTTPS, analytics receives test events, and a test signup is stored.
- [ ] **M0-06 · Apply to Paddle** · _you_ · §9.1, §9.6
    - Apply in week 1, with the landing page live.
    - Email support with these questions:
        - How are payouts made to an Indonesian bank?
        - In which currency? USD only
        - What are the payout fees and the minimum payout?
        - How long do payouts take?
    - Done when: the application is submitted and the answers are in the Decision log.
    - If the answer is "no", or still unclear at the end of week 2: start Plan B (§9.6) and check Indonesia payouts first.

### Accuracy spike (§5)

- [ ] **M0-07 · Accuracy test set** · _you_ (I give you the template) · §5.6
    - Template ready: `paccradle-backend/crates/vision/spike/truth.csv`, with instructions in `spike/README.md`. Photos go in `spike/photos/` (git-ignored).
    - 10 objects: flat box, jar, perfume bottle, tube, round item, white item, glass item, dark item, and 2 of your choice.
    - Measure the true size with calipers and write it into `paccradle-backend/crates/vision/spike/truth.csv`.
    - For each object take a **photo** and a **trace**, on A4 and Letter, with 2 phones, following the §5.1 capture rules.
- [x] **M0-08 · Spike: find and flatten the paper** · _claude_ · §5.3 steps 1–3 (done 2026-10-27)
    - CLI: `measure <photo> --paper a4|letter`, using `opencv` 0.101.x.
    - Steps: EXIF rotation, find the paper's 4 corners, sub-pixel corners, warp to 10 px/mm, save debug images.
    - Done when: a ruler photographed on the paper measures within ±0.5 mm after flattening.
- [x] **M0-09 · Spike: product outline + measurement** · _claude_ · §5.3 steps 4–8 (done 2026-10-27)
    - Product mask, shadow reduction, `minAreaRect`, circle fit, contour in mm, confidence score.
    - Done when: output JSON matches the §5.5 shape.
- [x] **M0-10 · Spike: Trace mode** · _claude_ · §5.2 fix A (done 2026-10-27)
    - Status (M0-08 to M0-10): built in `crates/vision`, with a `measure` tool that prints §5.5 JSON and debug images. 5 tests on generated photos pass (round, rotated box, trace, sideways Letter, no paper). Error is about ±0.25 mm: photo mode reads slightly big, trace mode slightly small. Real photos are checked in M0-11.
    - Detect the pen line, take its inner edge, and add back half the pen width (0.5 mm default).
- [ ] **M0-11 · Accuracy report** · _claude_ · §5.6
    - Tool ready: `scripts/in-docker.sh cargo run -p vision --release --bin accuracy`. Waiting on the M0-07 photos.
    - A script compares each result with `truth.csv` and reports median and P90 error for each mode.
    - Results go into `paccradle-backend/crates/vision/spike/REPORT.md`, compared against the §5.6 targets, with notes on what failed and why.

### Demand

- [ ] **M0-12 · Concierge designs** · _both_ · §12
    - 10–20 waitlist users send a photo. We return an insert made by hand: spike measurement + a hand-drawn dieline.
    - Ask each one: "Did it fit?" and "Would you pay $10 for the files?"
    - Done when: the answers are logged in the Progress log.
- [ ] **M0-13 · Demo videos · _you_ (I can write scripts) · §11.3, §11.4
    - 5 short videos, one of them on the landing page (generate via mcp fal ai image/video generator (Balance $1.35) or openrouter Image/video generator (Balance $10)).

### Gate 0

| Check                                      | Target                                                                    | Result |
| ------------------------------------------ | ------------------------------------------------------------------------- | ------ |
| Waitlist signups                           | ≥ 100 **or** ↓                                                            |        |
| Fake-door click rate (clicks ÷ visitors)   | ≥ 10%                                                                     |        |
| Accuracy (M0-11)                           | Within §5.6 targets, or within 2× with a known fix _(suggested bar)_      |        |
| Paddle                                     | Applied; Indonesia payout answer received (or Plan B started)             |        |
| Concierge users who said they'd pay        | _(record the number; no bar in the plan)_                                 |        |
| **Decision**                               | go / adjust / stop                                                        |        |

---

## M1: MVP build (weeks 3–8)

**Scope lock (§14):** 3 insert styles + 2 box styles + 3 export formats. **Nothing else.** Anything new goes to the Parking lot.

### M1-A · Geometry core `insert_core` (§6)

- [x] **A1 · Data model** · _claude_ · §6.1 (done 2026-10-28)
    - `Design`, `Material`, `Product`, `InsertSpec`, `Piece`, `Hinge`, all in mm, with serde.
    - Material defaults from §6.4.
    - Done when: JSON round-trip tests pass.
    - Status: built in `crates/insert_core/src` (`design`, `material`, `geometry`, `piece`, `limits`, `validate`). `Design::validate` checks every user value and names the JSON field that's wrong. The documented JSON example is `tests/fixtures/cradle_mailer.json`. 16 tests pass; fmt and clippy are clean. The WASM build isn't checked yet (target not installed; A10).
- [x] **A2 · Geometry helpers** · _claude_ (done 2026-10-28)
    - `kurbo` paths, polygon offset and boolean operations, simplification.
    - Try `i_overlay` and Clipper2 quickly, then pick one.
    - Done when: offset tests on rectangle, circle and concave shapes are within 0.01 mm.
    - Status: built in `crates/insert_core/src/geometry/`:
        - `flatten.rs`: `Path::flatten` (arcs to chords within a tolerance) and `Polygon::to_path`.
        - `simplify.rs`: `Polygon::simplify` (Ramer–Douglas–Peucker).
        - `area.rs`: `Area` with exact offsets (round outside corners), union, intersection and difference. The only file that uses `i_overlay`.
        - `Point::distance_to` and `Line::distance_to`, for A7.
    - Worst offset error is 0.002 mm on the rectangle, circle, L-shape and a dumbbell that splits in two when shrunk. Measured along every edge, not just at corners.
    - 28 new tests (44 in the crate). fmt, clippy and the whole workspace pass in Docker. `insert_core` now builds for `wasm32-unknown-unknown` (target installed on this Mac).
    - No `kurbo` (see the Decision log).
- [x] **A3 · Cutout card** · _claude_ · §6.2 (done 2026-10-28)
    - Card size, holes = outline + clearance, finger notches.
    - Status: `Design::generate_insert()` returns an `Insert` (pieces, length × width × height, and a placement for every product copy). Code in `crates/insert_core/src/insert/`:
        - `hole.rs`: hole = outline + clearance, plus a finger notch (19 mm wide) halfway along one long side.
        - `layout.rs`: the design's placements stay as they are. Copies without one are straightened and packed in rows with a §6.5 web (3 mm or 3× thickness) between holes. A3, A5 and the kit builder can share it.
        - `cutout_card.rs`: with a fixed box size, the card is that size minus the box's clearance on each side. Otherwise it grows around the holes plus `margin_mm`.
    - New geometry helpers: `Bounds`, `Transform`, `Polygon::convex_hull` and `straightening_turn_deg`, `Path::circle`, `Area::union_all`, `Material::min_web_mm`.
    - Errors: `DoesNotFit` (fixed box too small, with the size needed), `FallsApart` (holes cut the card in two), `NotBuiltYet` (grid and cradle, until A4/A5).
    - 26 new tests (70 in the crate). Holes match outline + clearance within 0.01 mm. Feeding the returned placements back rebuilds the same card. I checked rendered cards by eye (rows, notches, a product photographed at 30°, a fixed box).
    - Speed (native, release): 125 copies 14 ms; 1,250 copies 0.17 s.
    - Settings added the same day (your request), each defaulting to the centre:
        - `finger_notches`: `null` for plain holes, or `{ side, along }`. `side` is `bottom` (default), `top` or `both`, named as the product lies lengthwise and turning with it. `along` runs from 0 to 1 along that side (default 0.5, the middle).
        - `align`: where the generator's rows sit in a fixed box (`top_left` … `bottom_right`, default `center`). Rows inside the block line up the same way.
        - Per-product position was already a setting (`placements`); a new test checks that notches turn with a placed product. 78 tests in the crate.
- [x] **A4 · Partition grid** · _claude_ · §6.2 (done 2026-10-28)
    - Rows × columns, slot width = thickness + tolerance, slot depth = half height + 0.5 mm, optional frame, finger notches.
    - Done when: a test proves the crossing slots line up.
    - Status: `crates/insert_core/src/insert/partition_grid.rs`.
        - Long dividers (along x) are slotted from the bottom, short dividers (along y) from the top. Identical strips are one piece with a quantity.
        - A cell is the product's straightened footprint plus clearance. In a fixed box the cells stretch to fill it, and products turn crosswise if only that fits; otherwise `DoesNotFit`.
        - The frame adds strips round the outside. Every strip runs half a slot plus a web past the corner crossings.
        - Finger notches: one half-round notch in the top edge of each cell wall, up to 19 mm wide and at most half the strip height deep.
        - `Insert.mounts` (new, for every style) says where each cut copy stands, for the 3D preview (E6). The card gets one flat mount.
    - The line-up test works only from the cut outlines and the mounts. At every crossing it samples the full height: the two strips must never both have board, and each slot must clear a full board thickness. It covers 2 × 3 and 3 × 4 grids, with and without the frame and notches. Breaking the slots on purpose (1 mm too shallow, or 0.5 mm off position) makes it fail.
    - 13 new tests (91 in the crate). Workspace passes in Docker; WASM builds.
- [x] **A5 · Cradle platform** · _claude_ · §6.2 (done 2026-10-28)
    - From A3: add `align` to `CradlePlatformSpec` (the shared layout already takes it).
    - Deck, fold-down walls, lock tabs, holes taken at deck height. Emits hinges for the 3D fold.
    - Status: `crates/insert_core/src/insert/cradle_platform.rs`. One piece, 7 panels, 6 hinges, no glue:
        - **Deck** (panel 0, the fold's root): holes = outline + clearance, sized like the card (fills a fixed box, or grows round the holes). `align` added, default centre.
        - **Two legs**, one along each long edge (along x). Each folds down into a wall, in along the floor into a foot, and back up into a riser: three 90° mountain folds, stages 0–2. Lock tabs on the riser's top edge go up through slots in the deck and end flush with its top, which closes the leg into a tube.
        - **Sizes** follow from the board and the deck height (middle-plane lengths): wall = height − t, foot = half a slot + a web, riser = height − 1.5 t, tab = t tall and up to 15 mm long, about one per 60 mm (at least 2 when they fit).
        - Along the long edges, the holes keep clear of the legs and slots, which can be more than `margin_mm` (11.55 mm on chipboard).
        - "Holes taken at deck height": until side profiles arrive (v1), a product is a straight prism, so the outline at deck height is the top-view outline.
    - New error `DeckTooSmall` (deck too low for a riser, or too short for one tab); `NotBuiltYet` is gone. A hole across the whole deck gives `FallsApart { piece: "deck" }`.
    - Shared with the card now: `Layout::fixed_board` / `grown_board` with per-axis `Insets`, `fixed_room` and `one_region` (the grid uses the last two).
    - The main test folds the flat piece along its hinges in 3D, the way E6 will (`tests/common/fold.rs`), and checks that:
        - the feet rest on the floor;
        - every tab ends flush with the deck top and passes through a slot, its full thickness;
        - every slot gets a tab, and no other board pokes through the deck;
        - all board stays inside the footprint, and products stand clear of the legs.
    - It runs on 4 designs, including the documented fixture. Breaking the piece on purpose makes it fail:
        - slots 0.5 mm off;
        - a foot folded to 80°;
        - a wall folded the wrong way;
        - a crease off its edge.
    - I checked a rendered fixture by eye: the tabs line up with the slots.
    - 12 new tests (102 in the crate). fmt, clippy and the whole workspace pass in Docker; WASM builds.
- [x] **A6 · Outer boxes** · _claude_ · §6.3 (done 2026-10-29)
    - From A3/A4/A5: size the box from `Insert.length_mm`, `width_mm` and `height_mm`, and the tallest product. No generator checks product, divider or deck height against a fixed box height yet; do it here.
    - From A5: `tests/common/fold.rs` folds any piece along its hinges; reuse it for the box's fold test.
    - Mailer (FEFCO 0427) and shipping box (FEFCO 0201), sized around the insert, with hinges.
    - Status: `crates/insert_core/src/outer_box/`. The new `Design::generate()` returns a `Packaging`: the insert and, if the design has one, its box. In the formulas below, t is the box board's thickness and g its slot tolerance.
        - **Sizing** (`mod.rs`): inside = the insert's footprint plus the box's clearance on each side. Height = the tallest of the insert and the products, plus the clearance. A box with a fixed inside size keeps it.
        - New errors: `TooTallForBox` (a product, divider or deck taller than a fixed box) and `BoxTooSmall` (a fixed box below the style's smallest workable size). A box sized round the insert grows to that size instead.
        - **Mailer** (`mailer.rs`, FEFCO 0427 style, "roll end, tuck front"): one piece, 15 panels, no glue.
            - A base, single back and front walls with dust flaps, and double end walls (outer wall, rim, inner wall) that trap the dust flaps.
            - Lock tabs on the inner end walls go through slots in the base and end flush with its underside.
            - The lid hinges on the back wall and closes with a tuck flap.
            - Outside = inside + 6t + 2g long, 3t + g wide, 2t tall.
        - **Shipping box** (`shipping.rs`, FEFCO 0201): one piece, 13 panels.
            - Four walls in a row, with a joint flap glued or taped outside the back wall, so the inside stays clear.
            - Four flaps at the top and four at the bottom. The flaps on the longer walls meet in the middle; the shorter walls' flaps lie inside them.
            - Straight score lines: every wall is the same height, so the corners line up.
            - The outer flaps bend round the inner flaps (new `Hinge.offset_mm`): past the score, the board runs on one board thickness in the wall's plane, then the flap turns. Wall, bend and flap stay one board, and the outer flaps lie under the inner ones.
            - The bend uses up a board of each outer flap, so every flap is cut a board deeper than it reaches. The outer flaps meet in the middle with the slot tolerance between them, and all flap tips line up. Only in a nearly square box (sides within 2 boards) are the inner flaps cut shorter, so they don't run into each other.
            - Outside = inside + 2t long, 3t wide, 4t tall.
        - `src/folding.rs` (shared): builds a folding piece and places lock tabs. It was taken out of the cradle, which now uses it.
        - Box drawings are drawn from the inside: every crease is a valley fold, and the print side is the back of the drawing. The exports must mirror box pieces (B1–B4).
    - The fold test folds each box in 3D. `tests/common/fold.rs` now also handles pieces whose root stands upright (the shipping box's back wall), and the new `tests/common/solid.rs` turns folded board into blocks one board thick. It checks that:
        - no panel runs into another (the cradle's fold test now runs this check too);
        - nothing reaches into the inside;
        - each side of the inside is at least 90% closed, with board touching it;
        - the folded box measures the outside size the generator reports;
        - the mailer's tabs fill their slots and end flush with the base underside.
    - It runs on 6 mailers (the fixture, 4 boards, a long box with more tabs), 5 shipping boxes (longer length, longer width, square, 2 boards), and the smallest box of each style in paperboard and C-flute.
    - A sixth check: every folded panel still touches the panel it hinges on (nothing comes loose). The cradle's fold test runs it too.
    - 17 deliberate breaks (8 mailer, 9 shipping) are each caught by the check they target. For example: dust flaps 1 mm too deep collide; tabs 0.5 mm short aren't flush; top flaps cut to 20 mm leave the top open; a box 1 mm off the insert reaches into the inside; outer flaps folded on their crease (no offset) collide with the inner ones; an outer flap pulled 1 mm off its wall comes loose.
    - A test pins the shipping box's line-up. Flat: equal walls, equal flaps and level flap tips. Folded: level wall tops and bottoms, and outer flaps that bend down one board, stay joined to their walls and meet 0.5 mm apart.
    - Also tested: sizes round the insert, growing to the smallest size, fixed boxes, and both new errors.
    - I checked rendered drawings by eye: flat, closed and part-open, for the mailer and the shipping box both ways round.
    - Your review (same day): the shipping box's corners and flap tips didn't line up, because the walls carrying the outer flaps were drawn 2 boards taller. Fixed with straight scores and the hinge offset above; re-rendered and checked.
    - Your second review: the outer bottom flaps floated half a board below their walls, because the first fix moved them past the score without the board that bends there. Fixed by modelling the bend and cutting the outer flaps a board deeper. Rendered as solid board (top, part-open and from below): no holes, and the bottom closes.
    - 14 new tests (116 in the crate). fmt, clippy and the whole workspace pass in Docker; WASM builds.
- [x] **A7 · Manufacturability checks** · _claude_ · §6.5 (done 2026-10-29)
    - Every rule in §6.5 produces a warning that points to the piece or feature.
    - Includes open paths and duplicate lines.
    - From A3: the generator doesn't check the design's own placements against the card edge (only its own rows). The hole-to-edge rule covers them. Use `Material::min_web_mm` for the 3 mm / 3× thickness rule.
    - From A5:
        - Warn when a product is no taller than the cradle's deck. It would sink below the deck.
        - Warn when a placement puts a product over a leg; the generator only keeps its own rows clear.
        - The lock slots sit a web from the deck's creases on purpose. Don't flag them with the hole-near-crease rule.
    - From A6: the mailer's base slots sit 1.5 t + g/2 from the outer end walls' creases (2.65 mm in E-flute, under the 2 t rule) because the inner end wall stands there. Don't flag them either.
    - Status: `crates/insert_core/src/check/`. `Packaging::warnings((design, options))` returns a list of `Warning { rule, at }`, separate from generating.
        - `rule`: which rule and the numbers behind it (e.g. `holes_too_close` with `gap_mm` and `min_mm`). `Display` gives an English sentence; the editor translates by `rule.kind`.
        - `at` (a `Spot`): insert or outer box, the piece index, the box round the feature in that piece's flat drawing, and the product copies involved (indices into `Insert::placements`).
        - `CheckOptions.sheet`: the sheet or cutting mat to fit; no sheet skips that rule.
    - The rules, by module:
        - `holes`: holes closer than a web (3 mm or 3 t) to each other or to the piece's edge; product holes closer than 2 t to a crease. They use each product copy's own hole, rebuilt with the generator's code. So products that overlap (one merged hole) or run off the card (no hole at all) are caught, with gap 0. Holes holding no product are lock slots, and the crease rule skips them, which covers both exemptions above.
        - `board`: a thickness outside the board's usual range (§6.4), for the insert and the box; slots too tight or too loose (slot tolerance outside 0.1–1 mm, an estimate). Only for slots that take board: grid strips, the cradle deck, the mailer base.
        - `small_features`: board or cut-outs narrower than 1 mm. The outline and each hole are shrunk and regrown by 0.5 mm. Shallow steps don't count (a 0.5 mm paperboard tab on a wide panel is a step, not a strip). Thin board between two holes, or between a hole and the edge, is already a spacing warning with a stricter limit.
        - `cut_lines`: cut lines that don't close; lines drawn twice (one warning per pair of paths, with the shared length); creases that cross or end on a cut line away from their own ends.
        - `panels`: a panel more than 60 % holes (estimate); a piece bigger than the sheet either way round.
        - `cradle`: a product no taller than the deck; a product over a leg (its box reaching into the strip under each long edge, in to the riser's inner face).
    - Run over every generated sample (3 styles × 5 boards, both boxes): chipboard, E- and B-flute designs raise nothing. What it flags is real:
        - the sample card's gloss pinned off the card's edge;
        - C-flute cards with a 10 mm margin (C-flute needs a 12 mm web);
        - every slotted paperboard design: 0.8 mm slots, and the paperboard mailer's 0.55 mm slit between dust flap and end wall (see the Parking lot).
    - Speed (native, release): under 1 ms to about 25 ms for the samples; 30 ms for a card of 125 notched holes. Doing the shrink-and-grow on the whole board took 450 ms there. Now a quick test first finds the rings that could hold anything under 1 mm (two stretches close across but far apart round the ring), and only those are shrunk and regrown.
    - New public geometry helpers: `Polygon::contains`, `Polygon::edges`, `Line::distance_to_line`, `Bounds::gap_to`; `Bounds` now serializes.
    - 15 new tests: 12 for the checks (every rule fires on a design built to break it, with the right numbers, piece, spot and product copies; the default designs raise nothing; JSON shape) and 3 for the helpers. 131 tests in the crate. fmt, clippy and the whole workspace pass in Docker; WASM builds.
- [x] **A8 · 2D fit check** · _claude_ · §7.3 (done 2026-10-29)
    - From A7: `insert::placed_products` (crate-internal) gives every product copy's outline and its own hole, built the way the generator builds them. `check/holes.rs` shows how to tell lock slots (holes holding no product) from product holes. (Its distance helpers now live in `geometry/rings.rs`.)
    - Classify each hole green, yellow or red by clearance.
    - From A3: products that overlap share one merged hole, and a product placed off the card gets no hole. Both must show red. `Insert.placements` gives each product's position.
    - From A5: a cradle deck's holes and its lock slots are both in `panels[0].holes`. Tell them apart by which holes hold a placement.
    - Status: `crates/insert_core/src/fit.rs`. `Packaging::fits(design)` returns one `ProductFit` per product copy, in placement order: `color`, `clearance_mm` (the smallest gap), `target_mm` and `clashes`.
        - Green: at least the clearance recommended for the insert's board (§6.4), within the 0.01 mm accuracy bar. Yellow: fits, but tighter than that. Red: it runs into something, or comes within 0.01 mm of it.
        - A clash names what it runs into: `board`, another `product` (by placement), a cradle `leg`, or `off_insert` (hanging off the card or deck).
    - It looks at each product from above. What surrounds it:
        - every piece lying flat (the card, the cradle's deck), with the product in one of its holes;
        - every piece standing upright (grid strips), one board thick, placed by its mount;
        - the cradle's legs, under the deck's long edges;
        - the insert's own edge.
    - One method for all three styles: grid products are measured against their cell walls.
    - The notes above, covered: products in one merged hole are red and name each other, while still clearing the board. A product off the card is red (`off_insert`). Lock slots need no special case: they're holes in the deck like any other, and a product reaching one runs into the board round it.
    - What the samples show: every generated design is green at its clearance (paperboard and chipboard 1 mm, E-flute 1.5 mm). A paperboard card cut at 0.5 mm is yellow; at 0 mm it's red. A cradle product 0.5 mm from a leg is yellow; over the leg, red.
    - Speed (native, release): under 0.5 ms for the samples; 17.6 ms for a card of 125 notched holes, after measuring each product's own hole first so the others are skipped (69 ms before).
    - The exact ring distances moved from `check/holes.rs` to `geometry/rings.rs`, shared by the checks and the fit. New `Line::bounds`.
    - 7 new tests (138 in the crate), including two hand-broken packagings (a product on board with no hole; a grid strip moved into a row of cells). fmt, clippy and the whole workspace pass in Docker; WASM builds.
- [x] **A9 · Golden tests** · _both_ (done 2026-10-29)
    - Render every style to SVG snapshots for 5 sample products.
    - You look at them once, then the snapshots guard against regressions.
    - Status: 15 drawings in `crates/insert_core/tests/golden/`. Your review (same day): ok.
        - The samples: perfume (52 × 38 mm square bottle), candle (Ø72 jar), lipstick (Ø22 tube), soap (90 × 58 oval) and a pump bottle (Ø50 with a nozzle, not convex). Each goes in all 3 styles, each with a box. Together they cover all 5 boards, both boxes, frames, notch sides and an empty grid cell.
        - Each drawing shows every piece flat, with hinges coloured (blue valley, red mountain) and labelled with their stage. It also has a top view of the insert in the box with products coloured by fit, and text giving the sizes, fits and warnings.
        - The renderer is test code (`tests/common/svg.rs`), not the B1 export. A change to drawing styles doesn't touch geometry.
        - Numbers compare within 0.002 mm. The goldens pass on the Mac and in Docker (Linux).
        - A changed drawing fails its test, and the new one is written to the target folder. Accept intended changes with `UPDATE_GOLDEN=1 cargo test -p insert_core --test golden`.
    - **Bug found and fixed:** two samples didn't generate. A shipping box 118.953 mm long in E-flute "fell apart into 5 parts".
        - Each polygon boolean snapped to a 0.1 µm grid centred on its own shapes. Two panels built separately could put their shared edge one grid step apart, and joining them left a gap.
        - Booleans now use one grid whose origin is on a whole millimetre (`geometry/area.rs`), so a shared edge always snaps to the same line.
        - Offsets still use the library's own grid. They make new outlines rather than shared edges, so they aren't affected.
        - 2 regression tests: a row of 60 strips at odd positions, and boxes at 25 odd sizes. Both fail on the old code.
    - Also seen in the drawings, not changed:
        - The paperboard cradle's 0.8 mm lock slots warn (Parking lot item).
        - The pump bottle is turned about 25°. The layout turns each product to its smallest bounding box, which saves about 4% of board here.
    - 6 new tests (144 in the crate). fmt and clippy pass, the whole workspace passes in Docker, and WASM builds.
- [x] **A10 · WASM build** · _claude_ · §6.1, §7.4 (done 2026-10-29)
    - `wasm-bindgen` API. A feature flag keeps the exporters out of the WASM build.
    - The Next.js app imports the package built from `paccradle-backend/crates/insert_core` by local path. Nothing is published (DEC-03).
    - Preview coordinates rounded to 0.5 mm. (Changed after your review: see the status below.)
    - Done when: the frontend can call `generate()` quickly enough for slider drags. Record the bundle size.
    - From A7: expose `warnings()` as its own call. Run it after an edit settles, not on every slider frame (about 1–30 ms native).
    - From A8: expose `fits()` too. It's cheap enough to run with every preview (under 1 ms for normal designs), so products can change colour while they're dragged.
    - Status: the browser build is its own thin crate, `crates/insert_core_wasm`.
        - Why a separate crate: `wasm-bindgen`'s glue uses `unsafe`, which the workspace forbids, and a `cdylib` `insert_core` would slow every native build. `insert_core` stays plain Rust. This crate never enables the export writers, so they can't reach the browser (the "feature flag" item).
        - Three calls, JSON in and out: `generate(design)` gives the packaging rounded for the preview, `fits(design)` the product fits, `warnings(design, options)` the warnings. Each call regenerates from the design and keeps nothing. A design that can't be generated throws with the engine's message (e.g. "a design needs at least one product").
        - Preview geometry: `Packaging::rounded_for_preview` in `insert_core/src/preview.rs`. Every drawn line becomes plain straight segments within 0.05 mm of the true line, with points to 0.01 mm. Sizes, placements and mounts stay exact, because placements go back into the design while editing.
        - Your review (same day): with the first version, a 0.5 mm grid, curves were stair-stepped when zoomed, and the 6 copies of one hole each looked different. You chose smooth.
            - Closed outlines are now simplified to 0.05 mm before rounding to 0.01 mm. Checked in Chrome at 6× zoom: no steps, and the copies match.
            - A test pins it: copies of one hole draw within 0.15 mm of each other. The 0.5 mm settings gave 0.42 mm and fail it.
            - The preview is still not a production file: no arcs, no cut/crease layers, and DXF/PDF/clean SVG stay server-only, behind payment and with watermarks. CLAUDE.md §7.4 is reworded.
    - Build: `npm run build:wasm` in the frontend (runs `paccradle-backend/scripts/build-wasm.sh`) writes `wasm/insert-core/` (generated, gitignored).
        - It needs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129, the version pinned in the crate. The script checks the version.
        - CI's frontend job builds the package before `tsc` and `next build`. The `taiki-e/install-action` tool name is unchecked until the repo is on GitHub.
    - Frontend:
        - `features/design/insert-core.ts` loads the engine once, on first use, and returns `{ kind: "ok" | "error" }` results instead of throwing.
        - `features/design/types.ts` holds the engine's JSON types. `features/design/samples.ts` has the board starting values and a sample design; `features/design/svg.ts` has path data for SVG.
        - Next.js (Turbopack) serves the `.wasm` as a hashed static file. Marketing pages never reference it.
        - `/dev/geometry` is a live, timed check with a style picker, a size slider and a 200-frame benchmark. It's a 404 in production.
    - **Bundle size:** 615 KB `.wasm`, 218 KB gzipped (214 KB measured over the wire), plus 10 KB of JS glue.
        - Built with `opt-level = "s"`, LTO and the name section stripped.
        - Compared: opt-level 3 is 269 KB gzipped and about 30% faster; opt-level z is 186 KB and about 20% slower. Every level stays fast enough.
    - **Speed** (real Chrome on this Mac, 6 jars, one slider frame = `generate` + `fits`):
        - Grid: 2.2 ms median (95th percentile 3.6 ms).
        - Cutout card: 10.4 ms (13.9 ms).
        - Cradle: 7.4 ms (9.0 ms).
        - `generate` alone takes 1–1.8 ms in every style. Most of the time is `fits` rebuilding each product's hole (see the Parking lot).
        - `warnings`: 4–17 ms. Run it once an edit settles.
        - A card of 24 notched holes: about 45 ms per frame (Node).
    - 7 new tests: 4 for the preview geometry (`insert_core`, 148 tests) and 3 for the browser calls, run natively (`insert_core_wasm`). The whole workspace passes in Docker (156 tests), with fmt and clippy clean, including clippy for the WASM target. Frontend `tsc`, ESLint and `next build` pass.

### M1-B · Exports, server only (§6.6)

- From A7: pass the chosen sheet or Cricut mat as `CheckOptions.sheet` to warn about pieces that don't fit. There are no presets yet (U8).

- From A6: **box pieces are drawn from the inside**, so their print side is the back of the drawing. Mirror box pieces (x → −x) in every production file so the print side faces up, as printers expect. Inserts stay as drawn. Seen from the print side, a box's creases fold away from the viewer (mountain), so flip the fold direction too wherever a file marks it.

- [x] **B1 · SVG**: CUT and CREASE groups, mm units · _claude_ (done 2026-10-29)
    - Status: `crates/insert_core/src/export/`, behind the `export` feature. The API crate turns it on; the browser build never does, and the `.wasm` holds none of the exporter's text.
    - `Sheet::of(packaging)` lays out the drawing every production file shares. B2 and B3 draw the same `Sheet`.
        - Each piece appears once, insert pieces first, in rows that wrap at 1000 mm (a B1 press sheet), with a 10 mm margin and gap.
        - Box pieces are mirrored, print side up, and their arcs turn the other way. Inserts stay as drawn.
        - Lines are sorted into CUT (outline and holes), CREASE and PERFORATION, with the engine's exact geometry.
    - `Packaging::to_svg()` writes the file:
        - Canvas in mm, one user unit per mm, numbers to 0.0001 mm.
        - One layer per line kind, Inkscape-style: CUT `#ff0000`, CREASE `#0000ff`, PERFORATION `#00a000` (only when present). Strokes are 0.025 mm hairlines, so Epilog lasers cut instead of engraving.
        - Inside each layer, one group per piece, titled e.g. "Box: mailer x 1 (mirrored: print side up)". There's no drawn text, so a cutter never cuts a label.
        - Arcs stay arcs; one over a half turn is written as two halves.
        - Written by hand: plain text needs no crate (your vanilla-Rust rule), so CLAUDE.md's suggested `svg` crate isn't used.
    - Checked: both golden files are well-formed XML (`xmllint`). Rendered in Chrome: red cuts, blue creases, and the shipping box mirrored with its joint flap now on the left.
    - The golden-file comparison moved to `tests/common/golden.rs`, shared by the drawings (A9) and the exports.
    - 5 new tests (`tests/export_svg.rs`):
        - Every piece laid out once, in order, inside the margins, at least a gap apart.
        - Every line of every piece moved exactly, and mirrored for box pieces.
        - Arcs stay arcs and turn the other way when mirrored. A full circle is written as two halves.
        - The layers, units, path counts and piece titles, and no text.
        - Two whole files golden: `tests/golden/export_cradle_mailer.svg` and `export_grid_shipping.svg`.
    - 161 tests in the workspace in Docker, with fmt and clippy clean (with and without the feature). Run the export tests alone with `cargo test -p insert_core --features export`.
    - For you, once: open `tests/golden/export_cradle_mailer.svg` in Inkscape or your cutter app. CUT and CREASE should come in as separate layers or colours at true size (the cradle is 142 × 228.8 mm).
- [x] **B2 · PDF**: vector, CUT and CREASE layers, dimensions (`printpdf` or `krilla`) · _claude_ (done 2026-10-29)
    - From B1: draw `export::Sheet` (pieces placed, box pieces mirrored, lines sorted by `LineKind` with its layer name and colour). Labels and dimensions go on a layer of their own, never on CUT or CREASE.
    - Status: `Packaging::to_pdf()` in `crates/insert_core/src/export/pdf.rs`.
        - Written by hand, with no crate: it needs only lines, colours, layers and text in Helvetica, one of the fonts every PDF reader has built in. `krilla` would need a font file embedded just for labels. The file is uncompressed plain ASCII (8–12 KB for the samples).
    - The page:
        - One page at true size: 1 mm prints as 1 mm at 100 %. It's at least 150 mm wide, so the legend fits.
        - Laid out with `Spacing::DRAWING` (25 mm margin and gaps) to leave room for labels and dimensions. Cut files keep `Spacing::CUT_FILE` (10 mm).
    - Layers (optional content groups, shown as layers in Acrobat and Illustrator):
        - CUT: red, solid, 0.1 mm.
        - CREASE: blue, dashed 3/1.5 mm.
        - PERFORATION: green, dotted; only when a piece has perforations.
        - DIMENSIONS: each piece's length above it and width to its left, in grey with arrowheads and extension lines, to 0.1 mm.
        - LABELS: each piece's title (`SheetPiece::title()`, shared with the SVG) and a two-line legend: mm, print at 100 %, the line colours, box pieces mirrored.
    - Arcs become cubic Bézier curves of at most 45°.
    - Checked:
        - `pypdf` in strict mode reads both golden files: the 4 layers by name, the page size, and every label and dimension as text.
        - Rendered with macOS's PDF engine: dimensions, arrows, labels, the legend, dashed blue creases and the mirrored box. One crease looked dark grey at one render size; it is blue in the file and at other sizes, so it's the renderer's rounding.
    - 5 new tests (`tests/export_pdf.rs`):
        - Every object where the cross-reference table says, and the stream length right.
        - The page is the sheet at true size.
        - Each layer holds only its own lines (stroke count per layer), with no text on CUT or CREASE. Dimensions and titles are present.
        - Arcs: curves sampled at 5 points stay within 0.001 mm of the true circle, and turn the other way when mirrored.
        - Two whole files golden: `export_cradle_mailer.pdf` and `export_grid_shipping.pdf`.
    - The hand-built arc sample moved to `tests/common` (`arc_packaging()`), shared by both export tests. 166 tests in the workspace in Docker, with fmt and clippy clean; the WASM still holds none of the exporter.
    - For you, once: open `tests/golden/export_cradle_mailer.pdf` and print it at 100 % (actual size). The cradle should measure 142 × 228.8 mm on paper, and the layers should show in Acrobat or Illustrator.
- [ ] **B3 · DXF**: CUT and CREASE layers in mm · _both_
    - From B1: draw `export::Sheet` too, and write arcs as DXF arcs. The golden helper (`tests/common/golden.rs`) compares any text file.
    - Done when: the file opens correctly in LibreCAD or Inkscape and in one cutter app you use.
    - Status (2026-10-29): built; **waiting on your check in your apps**.
        - Try `crates/insert_core/tests/golden/export_cradle_mailer.dxf` and `export_grid_shipping.dxf` in LibreCAD or Inkscape and in your cutter app.
        - Check that CUT and CREASE come in as separate layers (or red and blue), and that the size is right: the cradle is 142 × 228.8 mm.
    - `Packaging::to_dxf()` in `crates/insert_core/src/export/dxf.rs`: the same `Sheet` as the SVG (`Spacing::CUT_FILE`).
        - DXF R12 (AC1009), the version nearly every program reads (LibreCAD, Inkscape, LightBurn, Silhouette Studio, ArtiosCAD). Written by hand, without the `dxf` crate.
        - Units: `$INSUNITS` 4 (mm) and `$MEASUREMENT` 1 in the header. y points up, as in CAD, so the sheet is flipped onto the bottom-left corner. `$EXTMIN`/`$EXTMAX` give the extent.
        - Layers: CUT red (colour 1), CREASE blue (5) and PERFORATION green (3), the last only when present. All continuous lines, never dashed.
        - Every path is one `POLYLINE`, closed where the path is, so a cutter follows it in one go.
        - Arcs are bulges, a quarter turn at most per stretch. Mirroring flips the bulge's sign.
        - No text: how many of each piece to cut is in the PDF.
    - Checked with `ezdxf` (a strict DXF reader): both golden files read as AC1009 in mm, and its audit finds 0 errors and nothing to fix. The layers, colours and polyline counts match.
        - Rendered: the same picture as the SVG, the right way up, with the shipping box mirrored.
        - A file with arcs (the hand-built sample) draws true circles. Its crease turns clockwise on the insert and the other way on the mirrored copy.
    - 5 new tests (`tests/export_dxf.rs`):
        - R12 in mm, with three sections and the extent; no text.
        - Layer colours; PERFORATION only when needed.
        - Every path one polyline on its layer, its points the sheet's with y flipped (within 0.0001 mm), closed flags matching.
        - Bulges rebuilt into stretches lie on the true circle within 0.001 mm, and flip sign when mirrored.
        - Two whole files golden: `export_cradle_mailer.dxf` and `export_grid_shipping.dxf`.
    - 171 tests in the workspace in Docker, with fmt and clippy clean; the WASM still holds none of the exporters.
- [x] **B4 · Print-at-home test PDF** · _claude_ · U3 (done 2026-10-29)
    - From A6: box pieces print mirrored (see the note above). The fold steps should say to fold away from the printed side.
    - Tiled across A4/Letter with overlap and alignment marks, fold steps, watermark.
    - Status: `crates/insert_core/src/export/test_print/`, behind the `export` feature. `Packaging::to_test_print(Paper::A4 | Paper::Letter)` returns the PDF. `TestPrint::of((packaging, paper))` is the layout behind it, public for tests.
        - **Every copy of every piece**, at true size, so the whole set can be built in one go. (Production files draw each piece once.) The order is the production sheet's: insert pieces, then box pieces mirrored, print side up.
        - **Layout** (`layout.rs`): each page's drawing area sits inside a 10 mm printer margin, above a 10 mm footer.
            - A piece that fits prints whole. Whole pieces share pages in rows, 6 mm apart, each with its label above.
            - A piece turns a quarter turn when only that fits, or when turning keeps it on the current row or page. The grid's three strips share one Letter page this way.
        - **Tiles**: a bigger piece gets pages of its own.
            - The windows overlap by 10 mm. They're the fewest that cover the piece plus 3 mm round it, centred on it, and turned whichever way needs fewer pages.
            - A tile the piece doesn't reach (beside a box's arm) isn't printed. The sample mailer takes 6 pages instead of 8.
        - **Joining tiles**:
            - Where a tile meets a printed neighbour on its left or top, that overlap is a hatched strip with a dashed cut line and "cut this strip off".
            - On its right and bottom, a solid line shows where the next tile's cut edge goes, with "B2 goes on top".
            - Grey diamonds straddle each seam. After cutting, the halves make whole diamonds only when the pages line up.
            - Tiles are named by row letter and column number (A1, B2) in each footer. The instructions draw a map of each piece's tiles.
        - **Fold steps**: each hinge's stage becomes a step number (1 first), shown in a circle on its crease.
            - White circle: fold away from the printed side. Black circle: fold towards it.
            - Box pieces print from the outside, so their valley folds (drawn from the inside) all read "away". `export::Fold` carries the hinges onto the sheet for this (`SheetPiece.folds`).
            - Circles that would overlap (creases a board apart, like the mailer's double end walls) move along their crease. They also keep out of tile overlaps when the crease allows.
        - **Instructions** (the first page or two), in English (DEC-11):
            - Print at 100 % and check the scale bar.
            - Join the pages (only when something is tiled).
            - Cut.
            - Fold, with each piece's steps taken from its hinges.
            - Slot the dividers together (grids only).
            - Check the fit, remembering that paper is thinner than board.
        - **Scale bar** in every footer: 50 mm on A4, 2 in on Letter. The file also asks PDF readers to print at actual size (`/PrintScaling /None`).
        - **Not a production file** (§7.4):
            - It draws the `rounded_for_preview()` geometry: straight segments, nothing the browser doesn't already get. There are no layers.
            - Each tile's lines are cut down to its window in Rust, so no page hides a whole tiled piece behind a clip.
            - Cut lines are black and solid (0.3 mm) and creases blue and dashed, so both read from a black-and-white printer.
        - **Watermark** (E9 asks that AI can't easily remove it):
            - "PACCRADLE TEST PRINT - NOT FOR PRODUCTION" in bold rows at 30° across the whole page, 20 % opaque, drawn over the lines. Removing it means redrawing the lines underneath.
            - Each page shifts its rows its own way, so there's no single repeating pattern to filter out.
            - No watermark stops a determined editor. The protection that matters is that the file is preview-grade and split into tiles.
        - The PDF code shared with B2 moved into `export/pdf/`: `draw.rs` (drawing commands, Helvetica widths) and `file.rs` (objects, cross-reference table, trailer). The production PDF's goldens are unchanged.
    - Checked:
        - `pypdf` in strict mode, with warnings as errors, reads both golden files: page sizes, print scaling, no layers, all text.
        - I rendered every page with macOS's PDF engine and looked at them: the instructions, the cradle, the mailer's tiles with seams and diamonds, and the grid on Letter.
    - 13 new tests (`tests/export_test_print.rs`):
        - Every copy is printed once, in order.
        - Whole pieces sit inside the window, 6 mm apart.
        - Tiles overlap by 10 mm, cover the piece and its pad, have no spare row or column, and join exactly their printed neighbours. Tiles left out hold no lines.
        - Pieces turn, both alone and to share a row.
        - Every drawn point lies inside its page's window.
        - Folds are numbered by stage, all go away from the print, and sit on their creases.
        - Circles stay clear of each other and of the overlaps. Turning off the spacing or the overlap rule on purpose fails this test.
        - Instruction sections appear only when they apply.
        - The watermark comes last on every page, shifted differently per page.
        - The file structure, the scale bars, and straight lines only.
        - Two whole files golden: `export_test_print_cradle_mailer_a4.pdf` and `export_test_print_grid_shipping_letter.pdf`.
    - The PDF test helpers moved to `tests/common/pdf.rs`, shared with the B2 tests.
    - 184 tests in the workspace in Docker, with fmt and clippy clean. The `.wasm` still holds none of the exporters (619 KB, 219 KB gzipped).
    - For you, once: print `tests/golden/export_test_print_cradle_mailer_a4.pdf` at 100 %. Check that the bar measures 50 mm, tape the mailer's tiles together using the map on page 1, and fold the cradle.
- [x] **B5 · Watermarked PNG preview** · _claude_ (done 2026-10-29)
    - Your choices (same day): drawn on the server with tiny-skia, showing the pieces and the fit view.
    - Status: `crates/insert_core/src/export/png/`, behind the `export` feature. `Packaging::to_png(&design)` returns the PNG. `Preview::of((packaging, design))` is the layout behind it, public for tests and later callers.
        - **What it shows**, top to bottom:
            - A title, then the sizes and the fit in words, e.g. "Fit: both products have at least the 1 mm room we suggest for chipboard", or "2 tight", "1 not fitting".
            - The top view: the insert in the box's inside, with products coloured by fit (green, yellow, red).
            - Every piece flat, once each, from the production sheet: insert pieces, then box pieces mirrored with the print side up. Cut lines solid, creases dashed, folds numbered in order.
            - A legend, and a footer saying it's a preview, not a production file.
        - **Scale**: the pieces share one scale, so their rows come out about 1800 px wide, at most 10 px per mm. The whole picture stays within 4000 px either way.
        - **The top view gets its own, larger scale** (about 720 px long) when the pieces' scale would leave it small, and its label then says "(enlarged)". Text keeps its size in px whatever the scale.
        - **Look**: the site's drawing colours from `styles/tokens.css` (espresso cuts, terracotta creases, kraft board, the three fit colours) on label cream, in Geist, the site's typeface.
            - The font is bundled at `crates/insert_core/fonts/Geist-Variable.ttf`, with its licence (SIL Open Font License 1.1) beside it, so the server needs no installed fonts.
        - **Fold circles** reuse the test print's placement, moved to `export/marks.rs` and now shared. Their size is a parameter, because the PNG sizes its circles in px.
        - **Watermark**: "PACCRADLE PREVIEW - NOT FOR PRODUCTION" in bold rows at 30° across the whole picture, 13 % opaque, drawn over the drawings. Each design shifts its rows its own way (seeded from the pieces' sizes), so the same design always draws the same picture. The row shift is shared with the test print (`export::jitter`).
        - **Not a production file** (§7.4): it draws the preview's geometry as pixels, nothing the browser doesn't already have.
        - New dependencies, server only (the browser build's tree has none of them): `tiny-skia` 0.12 (drawing, PNG writing) and `ttf-parser` 0.25 (letter shapes).
            - Counting their dependencies, 17 crates get compiled, mostly `png`'s compression and checksum stack (`flate2`, `fdeflate`, two `miniz_oxide` versions, `crc32fast`). That's more than the 5–8 I estimated when you chose this.
            - In dev and test builds the image crates compile optimised (workspace `Cargo.toml`). Unoptimised, one picture took seconds and the PNG tests 22 s; now 3 s.
        - Speed (native, release): about 180 ms per picture, about 590 KB each, for the two samples.
    - Checked: I looked at the rendered pictures: the cradle in a mailer, the grid in a shipping box, and a card with tight fits.
    - 8 new tests (`tests/export_png.rs`), reading the PNG back with tiny-skia's decoder:
        - A PNG of the laid-out size, at most 4000 px, with the top view first and then each sheet piece once, every drawing inside the picture and its label above it.
        - Each product shows its own fit colour where the layout puts it: green samples; a card cut tight (yellow); the sample card's gloss off the edge and a compact pushed onto it (red).
        - Every outline corner sits on a dark line, and creases show terracotta dashes.
        - The top view is enlarged to 720 px and says so; the pieces share the scale.
        - The watermark reaches every 400 px square.
        - Huge designs stay within 4000 px; tiny ones stop at 10 px per mm.
        - The same design gives the same bytes.
        - Two golden pictures, `preview_cradle_mailer.png` and `preview_grid_shipping.png`, compared pixel by pixel: at most 0.1 % of pixels may differ by more than 24 levels, because tiny-skia's SIMD smoothing can differ by a shade between processors.
    - 192 tests in the workspace in Docker, with fmt and clippy clean. The `.wasm` is unchanged: 619 KB, 219 KB gzipped, with no PNG code or font in it.

### M1-C · Vision service (§5)

- [x] **C1 · Production vision crate** · _claude_ (done 2026-10-29)
    - Turn the spike into the `vision` crate. It runs as a job and returns §5.5 JSON.
    - Status: `crates/vision`. The capture job (D5) calls one function: `vision::run((&input, &image_bytes))`. It returns the §5.5 `VisionResult`, or a `VisionError` with a stable code. The job table and the API's link to OpenCV come with D2 and D5.
        - **Input** (`input.rs`): `CaptureInput { paper, mode, pen_width_mm }` reads from the job's JSON. `paper` takes `A4` or `a4`, `Letter` or `letter`; the pen width defaults to 0.5 mm and must be 0.1–3 mm. It's checked before the file is looked at.
        - **Upload checks** (`image.rs`): `vision::inspect(bytes)` reads the format and pixel size from the file's header, decoding nothing.
            - Only JPEG, PNG and WebP (lossy, lossless and extended) are accepted. HEIC is converted to JPEG in the browser (E2).
            - At most 40 million pixels, so a crafted file can't make the server decode a huge image (§8.5). At least 1000 px on the long side, about the least that measures to the §5.6 targets.
            - The API can call it at upload time (D4).
        - **Decoding**: `vision::decode` runs the checks, then decodes to BGR, turned upright by the photo's EXIF orientation. OpenCV 4.10 does that when decoding from memory too; a test checks it. It refuses images whose pixels don't match their header.
        - **Errors**: each has a stable code the API translates: `CAPTURE_UNSUPPORTED_IMAGE`, `_UNREADABLE_IMAGE`, `_IMAGE_TOO_LARGE`, `_IMAGE_TOO_SMALL`, `_INVALID_INPUT`, `_PAPER_NOT_FOUND`, `_PRODUCT_NOT_FOUND`, `_TRACE_NOT_FOUND` and `_PROCESSING_FAILED`. `is_users_to_fix()` is false only for the last, an OpenCV failure the API logs and doesn't explain.
        - **Warnings** are codes now, translated by `kind` like the A7 warnings: `tall_product_looks_bigger`, `touches_paper_edge`, `other_objects` (with `count`) and `low_confidence` (with `confidence`). The English sentences stay as `Display`. CLAUDE.md §5.5's example is updated.
        - **The result reads back from JSON**, because the API stores it with the capture and reads it again. A test checks the round trip and the §5.5 shape.
        - Both tools (`measure`, `accuracy`) now read photos the job's way. The accuracy report (M0-11) calls `vision::run` itself, so it measures exactly what production runs.
        - The library docs no longer call it a spike. The `spike/` folder keeps the M0-07 photos, truth table and report.
    - Speed: about 0.5 s per 4000 × 3000 JPEG photo, from bytes to result (release, Docker x86_64 on this Mac).
    - 8 new tests (`tests/production.rs`, Docker only, like all OpenCV tests):
        - `run` measures the same disc from JPEG, PNG and WebP files.
        - `inspect` reads all four header kinds.
        - Headers claiming 400 million, 81 million and 100 million pixels are refused without decoding.
        - Small, foreign (GIF, HEIC, PDF, empty), cut-short and broken files are refused with the right codes.
        - EXIF orientation 6 turns the photo upright and it still measures.
        - The input reads from JSON and bad pen widths are refused first.
        - The result has the §5.5 shape and reads back.
        - The error codes and who can fix each.
    - The synthetic-photo helpers moved to `tests/common`, shared with the spike's 5 tests. 13 vision tests pass in Docker; fmt and clippy are clean.
- [ ] **C2 · Trace mode in the production path** · _claude_
    - Includes the pen-width setting.
    - From C1: trace mode already runs through `vision::run`, with the pen width checked (0.1–3 mm, default 0.5). What's left needs the M0-07 photos: trace-specific checks on real photos (gaps in the pen line, a product left on the paper), and calibrating the half-pen-width rule (M0-10). The editor's pen-width setting is E2.
- [x] **C3 · Manual corrections** · _claude_ · §3 step 4 (done 2026-10-29)
    - A tap point re-runs detection. An edited contour is accepted as it is.
    - Status: both speak **photo pixels**, the frame of `corners_px`. Every detected product now also carries its outline in photo pixels (`contour_px`, the same points as `contour_mm`), so the editor draws it over the photo and drags its points with no maths of its own. `paper::PaperMapping` maps between the photo and the flattened paper.
        - **Tap**: `CaptureInput.tap_point_px` (absent on a first run). `vision::run` finds the paper again, maps the tap onto it and keeps the outline holding the tap. A tap that just misses picks the nearest outline within 10 mm. The other objects then don't lower the confidence or raise `other_objects`: the user has chosen. Works in photo and trace mode, and after `CAPTURE_PRODUCT_NOT_FOUND`, since a tap only needs the photo.
            - Refused: a tap on bare paper or the table → `CAPTURE_NOTHING_AT_TAP`; outside the photo or not a number → `CAPTURE_INVALID_INPUT` (`tap_point_px`), the second before the file is read.
        - **Edited outline**: `vision::accept_edit((&stored_result, &points_px))`. It needs no photo and nothing is detected or simplified: the points are mapped onto the paper and measured as drawn. The product gets `contour_source: "user"` (detected ones say `"detected"`) and confidence 1. Paper, mode and level stay. Only the parallax warning is kept (photo mode), since the others are about finding the product.
            - Refused: fewer than 3 or more than 2,000 points, a point that isn't a number or lies off the paper, less than 25 mm² → `CAPTURE_INVALID_INPUT` (`edited_contour_px`); an outline that crosses or touches itself → `CAPTURE_OUTLINE_CROSSES_ITSELF`. Repeated points and a closing repeat of the first are dropped.
        - The `measure` tool takes `--tap X,Y` for trying taps on the M0-07 photos.
    - Sent back unchanged, a detected outline measures a little smaller than detected: 80.25 × 30.26 mm became 80.08 × 30.02 on the synthetic 80 × 30 box. Detected sizes come from every edge pixel plus half a pixel; the outline keeps fewer points (ε 0.2 mm). Here the outline was the closer one (Parking lot).
    - 13 new tests (26 in the crate, Docker): 9 in `tests/corrections.rs` (taps in both modes, near misses, refused taps, the tap through `run`'s JSON input, `contour_px` lying on the product's edge, an edited rectangle measured within 0.1 mm, an unchanged outline, refused outlines) and 4 unit tests of the crossing check. 213 tests in the workspace; fmt and clippy clean.
- [x] **C4 · Tall-product warning + height input** · _claude_ · §5.2 fix B, DEC-09 (done 2026-10-29)
    - Built for DEC-09's suggestion, **the user types the height**; the lay-down photo stays in M3-03. DEC-09 stays open for your confirmation.
    - **Height**: `CaptureInput.height_mm` (so a re-run after a tap keeps it), or `vision::set_height((&stored_result, height_mm))` for a result that already exists. `set_height` needs no photo. The height must be 0.1–1000 mm; otherwise `CAPTURE_INVALID_INPUT` (`height_mm`). It lands on the product's `height_mm`.
    - **Camera** (`camera.rs`): every result now has `camera: { height_mm, foot_mm, focal_source }`. `height_mm` is the lens's height above the paper and `foot_mm` the point straight below it, worked out from the paper's 4 corners and the lens.
        - The lens comes from `CaptureInput.focal_length_35mm`, the EXIF `FocalLengthIn35mmFilm` the browser reads (5–3000 mm, else `CAPTURE_INVALID_INPUT`). Without it, `focal_source` is `"assumed"` and a 24 mm lens is used. That's wider than most phone lenses, so the camera comes out nearer than it was and the warning errs on the safe side.
        - On exact test photos: 350.01 mm found for a true 350 mm (level), 300.1 for a true 300 mm tilted about 31°, foot within 0.3 mm. Assuming 24 mm for a 26 mm lens gives 323 mm.
    - **Warning** (`height.rs`), photo mode only (a trace lies on the paper):
        - No height yet: `tall_product_looks_bigger`, as before (now "…or enter its height to see by how much").
        - With a height: `outline_may_be_bigger { height_mm, up_to_mm }`. `up_to_mm` is how far the outline may reach past the product's real edge, per side: height ÷ camera height × the outline's farthest distance from the point below the lens. It's exact for an upright cylinder and an upper bound for anything narrower at the top. Below 0.5 mm there's no warning.
        - On a drawn 52 mm cylinder 80 mm tall (measured 67.5 mm), the outline reached 7.76 mm past the edge and the warning said 7.8 mm. Tilted, 60 mm tall: 51.9 mm reached, 51.9 mm warned.
        - Example: a 30 mm tall jar 35 cm below the lens gets "up to 2.3 mm"; a 1 mm card gets nothing.
        - `set_height`, `accept_edit` and `run` all go through one function, so they agree. Changing the height drops the old tall-product warning and keeps the others. An edit keeps the height and re-checks the warning for the new outline.
    - Tools: `measure --height-mm H --focal-35mm F`. The accuracy report passes each photo's height from `truth.csv` and shows the warning's bound next to the error (lens assumed: it can't read EXIF from the files).
    - 8 new tests (`tests/height.rs`, on photos from a known pinhole camera, `common::Pinhole`): the camera level and tilted, the assumed lens, the cylinder's reach against the warning, a low product, `set_height` equal to a run with the height (other warnings kept, bad heights refused), a trace, an edit keeping the height, the JSON and the input checks. 34 in the crate, 221 in the workspace; fmt and clippy clean.
- [ ] **C5 · Accuracy regression in CI** · _claude_
    - The M0-07 photo set runs in CI. The build fails if median error gets worse.

### M1-D · Backend API (§8, following your backend rules)

- [x] **D1 · API skeleton** · _claude_ (done 2026-10-29)
    - Typed config, `AppError` with error codes, locales, tracing.
    - tower-http middleware: request ID, CORS, compression, body limit, timeouts, rate limit.
    - Health route.
    - **Surfaces** (your request): `/api/client/v1` for the web app's users, `/api/admin/v1` for the back office, `/api/webhooks/v1` for Paddle (F3), and unversioned `/healthz` + `/readyz` for the deploy. Each surface has its own router, CORS origin (`APP_WEB_URL`, `ADMIN_WEB_URL`) and contract version.
    - **Semver per surface**: the URL holds the major, the `api-version` header the full version (`1.0.0-beta.1` until launch); `GET /api/{surface}/v1/version` returns it. Rules in the backend architecture rules §4.8.
    - The admin surface fails closed: every admin path, unknown ones too, answers 401 `AUTH_REQUIRED` until admin sign-in exists (D3).
    - Config: `AppConfig::load()` reads the environment, then `./.env` (own reader, Docker `--env-file` rules). Production needs an `https://` `APP_WEB_URL`. New settings in the env examples: `APP_WEB_URL`, `ADMIN_WEB_URL`, `HTTP_BODY_LIMIT_BYTES` (1 MiB), `HTTP_REQUEST_TIMEOUT_SECONDS` (30), `RATE_LIMIT_PER_MINUTE` (600), `RATE_LIMIT_CLIENT_IP_HEADER`, `RUST_LOG`.
    - Errors: `AppError` → `ErrorReport` in the response → the `error_body` middleware writes `{ error: { code, message, meta } }` in the caller's language. Framework errors (bad JSON, 405, panics) get our codes too. 11 codes so far.
    - Middleware, outermost first: request ID (UUID v7, the caller's is dropped), trace (JSON lines in production, no query strings), compression (gzip/brotli, not for responses that set a cookie: BREACH), error body, panic catcher, 30 s timeout, per-IP token bucket, 1 MiB body limit.
    - Deploy scripts, CI smoke test and docs now call `/readyz` and `/healthz`; the Paddle webhook path is `/api/webhooks/v1/paddle` everywhere. CLAUDE.md §8.3 lists the endpoints per surface.
    - Contract: `postman/api-skeleton-2026-10-29.postman_collection.json`.
    - 30 tests (`tests/api_surfaces.rs`, `middleware.rs`, `config.rs`, `shared.rs`) pass on the Mac and in Docker; fmt and clippy (`-D warnings`) clean. The server ran locally: SIGTERM stops it cleanly.
- [x] **D2 · Database** · _claude_ · §8.4 (done 2026-10-29)
    - sqlx migrations for every §8.4 table: UUID v7, `timestamptz`, money in minor units.
    - Job/outbox table and worker: `FOR UPDATE SKIP LOCKED`, retries, dead letter.
    - From D1: `/readyz` adds a Postgres ping (503 `SERVICE_UNAVAILABLE` when it fails; the deploy waits on it). The pool joins `AppState`. CI's `cargo sqlx prepare --check` step needs sqlx and `migrations/` from here on.
    - **Migrations** (`migrations/`, 5 files): `users`, `captures` + `products`, `designs` + `exports`, the billing tables (`design_unlocks`, `credit_ledger`, `subscriptions`, `paddle_events`), `jobs`. Conventions in the first file's header: UUID v7 made by the API, `timestamptz`, `updated_at` by trigger, checked lower-case text for enum-like columns, `ON DELETE CASCADE` from users, add-only migrations (a rollback still runs).
    - **Guests:** `users.email` may be NULL, so visitors can capture and design before signing in (when sign-in is asked for is D3's call). Emails are unique whatever their case.
    - **Columns beyond §8.4** (CLAUDE.md §8.4 updated): `captures.status`, `input` (the vision input), `error_code`; `designs.name` (for "My designs"); `credit_ledger.reason` (purchase, spend, refund, chargeback, adjustment) and `note`; `subscriptions.paddle_updated_at` (to skip late events); an `id` on `design_unlocks` and `paddle_events` (your UUID-id rule) next to their unique keys. No money columns: Paddle keeps the amounts, so the minor-units rule waits for the first one.
    - **Rules Postgres enforces:** the credit ledger is append-only (a trigger), a purchase's credits are added once per Paddle transaction, a resent Paddle event is stored once, a running job always has a lease, and unknown enum values are refused.
    - **Job queue:** `JobService` (enqueue inside the caller's transaction, claim, finish, tidy) on `PgJobRepository`; `jobs::Worker` with `JOB_WORKERS` claim loops (default 2, 500 ms poll) and a housekeeping loop. 5-minute leases; retries after 10 s, 20 s, 40 s… up to 1 h; 5 attempts by default; `GiveUp` and unreadable payloads go straight to dead letters; a panicking handler counts as a retry; done jobs go after 7 days. Workers only claim kinds they have a handler for (none yet), so a rollback leaves newer jobs waiting.
    - **Startup:** connects (5 s acquire timeout, 30 s statement timeout), applies migrations when `DATABASE_RUN_MIGRATIONS` (default true), starts the workers; SIGTERM stops claims and gives running jobs 25 s. New settings: `DATABASE_URL` (required in production; development default is `scripts/dev-db.sh`'s container), `DATABASE_MAX_CONNECTIONS`, `DATABASE_RUN_MIGRATIONS`, `JOB_WORKERS`. The URL is a `Secret`: it never shows in logs or `{:?}`.
    - `scripts/dev-db.sh` starts Postgres 18 on 127.0.0.1:54329; `.sqlx/` holds the checked queries for offline builds (the Dockerfile sets `SQLX_OFFLINE=true`).
    - Contract: `postman/readiness-database-2026-10-29.postman_collection.json` (`/readyz` can now answer 503).
    - 52 tests (22 new: `tests/schema.rs`, `jobs.rs`, `readiness.rs`, two in `config.rs`) pass against Postgres 18; fmt and clippy (`-D warnings`, offline) clean. A real run on a fresh database applied the 5 migrations, answered `/readyz` 503 with the database stopped and 200 once it was back, and stopped cleanly on SIGTERM.
- [x] **D3 · Auth** · _claude_ · DEC-07 (done 2026-10-29)
    - user required register Passkey if their device support (after success register or login with password), so next time when login , they can use passkey easy, but fallback if fail using passkey 2 times tried, we can send token magic link to email if they forgot their password.
    - when register to our platform, make sure they need confirm email when we send magic link
    - From D1: replace `middleware::admin_guard::require_admin` (refuses everyone now) with a real check. Suggested: an admin role on the user, passkey required for admins with no magic-link fallback, short sessions, and a log of every admin write (who, what, when). The admin surface has its own CORS origin, `ADMIN_WEB_URL`.
    - From D1: responses that set the refresh cookie are never compressed (BREACH); keep every token-carrying response on one of those.
    - From D2: decide when sign-in is asked for. The schema allows guests (`users.email` NULL): a visitor can capture and design first, and signing in puts the email on the guest's row (or merges it into an existing account). The sign-in tables (magic links, refresh tokens, passkeys, password hashes, email confirmed at, the admin role) are a new migration here. Token expiry is the first `timestamptz` read into Rust: pick sqlx's `time` or `chrono` feature then.
    - **Built** (rules: `backend-architecture.md` §8; contract: `postman/auth-2026-10-29.postman_collection.json`; client and admin contracts now `1.0.0-beta.2`):
        - **Sign-up = confirm the email first.** `POST /auth/email-link` sends "confirm your email" (no account; 24 h) or a sign-in link (account; 15 min), with the same 202 either way. The link (`{APP_WEB_URL}/auth/link#token=…`) confirms the address and signs in; only then is the email stored. The password comes right after (`PUT /me/password`, no current password needed within 15 minutes of a link or passkey sign-in). Asking for the password before the confirmation would let someone sign up with your address and a password they know, then wait for you to click (account pre-hijacking).
        - **Forgot password** is the same link: sign in with it, set a new password. Every other session of the user ends when the password changes.
        - **Passkey after sign-up or a password sign-in**: every session answer carries `next_steps` (`set_password`, `add_passkey`). `add_passkey` follows any sign-in that didn't use a passkey; the web app offers it where the device can make one (the server can't know). Sign-in: passkey first; after 2 failed tries the web app shows the password form and "email me a link" (E1).
        - **Guests** (D2's question): `POST /auth/guest` starts one. Confirming an email from the guest's browser turns that guest into the account; signing in to an existing account from it moves the guest's captures, products and designs over and deletes the guest. Only the browser's own cookie decides this, never the link. When sign-in is asked for is DEC-14.
        - **Sessions**: HS256 access token (15 min; 5 on the back office) + refresh token in `paccradle_refresh` (`HttpOnly; SameSite=Strict; Secure` on HTTPS; `Path=/api/client/v1/auth`), 30 days sliding, stored as SHA-256 and replaced on every refresh. A token reused within 30 s (two tabs) gets an access token and no new cookie; later, it ends the session. `/auth/…` refuses another site's `Origin`, and sign-ins share `RATE_LIMIT_AUTH_PER_MINUTE` (20) per address; password tries are also capped at 5 a minute per email, and links at 3 per 10 minutes per email.
        - **Passkeys**: W3C JSON in and out; ES256, EdDSA and RS256; user verification required; attestation not checked; challenges single-use for 5 minutes. Checked by hand on `ring` with a small CBOR reader (`shared::webauthn`): the WebAuthn crates pull in OpenSSL.
        - **Back office** (from D1): `require_admin` now lets through an admin access token from the admin surface's own passkey sign-in (`/api/admin/v1/auth/passkey/sign-in`; no password, no link). 12-hour sessions that refreshes don't extend; losing the role ends the session at its next refresh. Every admin write is logged in `admin_audit_log` before it runs (admin, email, method, route, path, request id, then status); a trigger keeps the log append-only. The role is granted in SQL for now: `UPDATE users SET role = 'admin' WHERE email = '…'` after the person has made a passkey in the web app.
        - **Email**: SMTP through `lettre` (STARTTLS, TLS, or none for Mailpit only). The `send_email_link` job makes the token when it sends, so no token or address sits in the queue. `scripts/dev-mail.sh` starts Mailpit (inbox at http://localhost:8025).
        - **From D1 and D2**: token-carrying responses set the cookie or are `no-store`, and neither is compressed. Timestamps are read with the `time` crate.
        - **Migration** `…130000_auth.sql`: `users.role`, `email_confirmed_at`, `password_hash`; `sessions`, `email_links`, `passkeys`, `passkey_challenges`, `admin_audit_log`. An email is now stored only once confirmed (a CHECK); D2 never wrote one, so a rollback still runs.
        - **New settings**: `AUTH_JWT_SECRET` (required in production; the template's placeholder is refused), `WEBAUTHN_RP_ID` (default: `APP_WEB_URL`'s host; must cover `ADMIN_WEB_URL`'s too), `WEBAUTHN_RP_NAME`, `RATE_LIMIT_AUTH_PER_MINUTE`, `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURITY`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `EMAIL_FROM` (SMTP and sender required in production). The production env template said `APP_ENV=development`; now `production`.
        - **Tests**: 34 new (`tests/auth.rs` 16 end to end on Postgres, `webauthn.rs` 8 against a software authenticator with P-256, Ed25519 and RSA keys, `auth_primitives.rs` 10); 86 in the API crate. fmt and clippy clean; the whole workspace passes in the Docker toolchain too. A real run sent the confirmation email through Mailpit, then link sign-in, password, password sign-in and refresh worked; no token, cookie or password reached the log.

- [ ] **D4 · Uploads & storage** · _claude_ · §8.5
    - From D3: the upload handler takes `AuthUser` (guests too). A per-user upload limit can key `RateLimiter` on `user_id`.
    - Cloudflare R2 provider behind a trait.
    - 10 MB limit, file-type check, pixel cap, rate limits.
    - Strip GPS from EXIF, signed URLs, a job that deletes photos after 30 days.
    - From C1: `vision::inspect(bytes)` does the file-type check and pixel cap from the header, without decoding (JPEG, PNG, WebP; at most 40 million pixels, at least 1000 px on the long side). Call it before storing. It doesn't strip GPS: a photo re-encoded through the browser's canvas has no EXIF left, but one sent straight from a phone still does.
    - The docs disagree on the upload size: 10 MB here, 15 MB in CLAUDE.md §8.5 and the handler rules. Pick one here.
    - From D2: the purge is a `purge_photos` job kind; it deletes the R2 object, then sets `captures.storage_key` to NULL (the `captures_purge_due` index finds them). The queue has no schedule yet: add a daily enqueue (one job per day, skipped if one is already queued).
    - From D1: raise the body limit on the upload route only (`DefaultBodyLimit::max` on that route beats the global 1 MiB), and give it a longer timeout than the global 30 s (Nginx allows uploads 150 s). Nginx's upload location still matches `/api/v1/progress-photos/…/file` from another project: point it at `/api/client/v1/captures`.
- [ ] **D5 · Captures endpoints**: create, get, correct; each one starts a vision job · _claude_ · §8.3
    - From C1: the job calls `vision::run((&input, &bytes))` in a blocking task (about 0.5 s a photo) and stores the `VisionResult` as the capture's `result` jsonb. On failure it stores the error's `code()`: explain it to the user when `is_users_to_fix()`, log it when not. The API then links OpenCV, so the runtime image needs its libraries (Dockerfile note).
    - From C3, `POST /captures/{id}/correct` takes `{ "tap_point_px": [x, y] }` or `{ "edited_contour_px": [[x, y], …] }`, both in photo pixels:
        - A tap starts a vision job with the capture's stored input plus `tap_point_px` (the same `vision::run`). It works after `CAPTURE_PRODUCT_NOT_FOUND` too.
        - An edit calls `vision::accept_edit((&stored_result, &points))`. There's no photo and at most a few milliseconds of work, so it can answer within the request instead of a job. Your call here.
        - Until L2 exists (M3), a retry without a tap gives the same result, because L1 always does the same thing. Offer a tap, Trace mode or an edit instead (§5.4 retries).
        - New codes to translate: `CAPTURE_NOTHING_AT_TAP`, `CAPTURE_OUTLINE_CROSSES_ITSELF`; `CAPTURE_INVALID_INPUT` can now name `tap_point_px` or `edited_contour_px`.
    - From D2: create the capture row and its `run_vision` job in one transaction (`JobService::enqueue(&mut tx, …)`). Column values are lower case (`mode` photo/trace, `paper` a4/letter, `level_used` l1); `status` pending → done/failed; a re-run after a tap keeps the old `result` until the new one lands. `purge_after` = created + 30 days.
    - From D1: the routes go on the client surface (`/api/client/v1/captures…`). Map every `vision` error code into `ErrorCode` with its English message in `locales/en.rs`, and bump the client contract version.
    - From C4: store `focal_length_35mm` (sent by E2) in the capture's input, and a typed height as `height_mm`, so every run uses them. When the height is typed after the photo (E4), `correct` can take `{ "height_mm": 95 }` and call `vision::set_height((&stored_result, h))`: no photo, no job. The design's product height (§8.4 `products.height_mm`) is the same number. `CAPTURE_INVALID_INPUT` can name `height_mm` or `focal_length_35mm`.
- [ ] **D6 · Designs endpoints** · _claude_ · §8.3
    - Create, get and patch, with versions.
    - From D2: `designs.version` starts at 1; bump it on every saved change (an export records the version it was made from). `name` (≤ 200 characters) is there for the list's search and sort.
    - "My designs" list with `page`, `page_size`, `total_pages`, filtering and sorting (your code-style rule).
- [ ] **D7 · Exports endpoints** · _claude_ · §9.3
    - From D3: exports need an account (DEC-14): a guest gets `AUTH_ACCOUNT_REQUIRED` (403), and the web app opens sign-up. Billing rows must never belong to a guest: the guest merge moves only captures, products and designs, and deleting a guest would cascade its ledger rows away.
    - Entitlement check: Pro, then credit, otherwise `402` with the checkout payload.
    - Export job, then signed download URLs.
    - From B4: the test print (`Packaging::to_test_print(paper)`) is free, so it skips the entitlement check. Rate-limit it like other exports. `Paper` has no serde yet: add it with the request schema.
    - From B5: the PNG preview (`Packaging::to_png(&design)`) is free too, about 180 ms per picture: run it as a job like the other exports.
    - From D2: spending a credit locks the user's row (`SELECT … FOR UPDATE`), checks `SUM(delta)` > 0, then writes the `spend` row, the unlock and the export with its job in one transaction; Postgres doesn't stop a negative balance by itself. `exports.formats` takes svg, pdf, dxf, png, test_print.
- [ ] **D8 · `/me` endpoint** · _claude_
- [ ] **D9 · Tests + contracts** · _claude_
    - `#[sqlx::test]` integration tests for every endpoint.
    - A dated Postman contract file for every endpoint (your rule).

### M1-E · Frontend (§7: Next.js, no Tailwind, dependencies per DEC-05)

- [ ] **E1 · App shell** · _claude_
    - Routes, a typed `fetch` hook for API calls, auth screens, `t()` for strings, design tokens.
    - From D3 (contract: `postman/auth-2026-10-29.postman_collection.json`):
        - Fetch with `credentials: 'include'`; keep the access token in memory only. On `AUTH_REQUIRED`, call `POST /auth/refresh` once and retry; serialise refreshes across tabs (`navigator.locks.request('refresh', …)`). `AUTH_SESSION_EXPIRED` means sign in again. On start-up, refresh; with no session, start a guest (`POST /auth/guest`) at the first action that needs one.
        - Sign-in screen: passkey first (also as autofill: `mediation: 'conditional'` with `autocomplete="username webauthn"`). Count `NotAllowedError` and `AUTH_PASSKEY_REJECTED` as failures; after 2, show the password form and "Email me a sign-in link". Sign-up and "forgot password" are the same call (`POST /auth/email-link`); say "check your inbox" either way.
        - The `/auth/link` page reads `#token=` from `location.hash`, clears it (`history.replaceState`), and posts it to `/auth/email-link/sign-in`.
        - After a sign-in, follow `next_steps`: `set_password` → the password screen (12 to 128 characters; no current password needed for 15 minutes); `add_passkey` → only when `PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()` is true. `InvalidStateError` from `create()` means this device has one already: treat it as done. Let the user skip it "on this device" (remember that locally).
        - Passkey JSON is the W3C shape: `PublicKeyCredential.parseCreationOptionsFromJSON` / `parseRequestOptionsFromJSON` and `credential.toJSON()`, with a small base64url fallback for browsers without them.
        - The web app must be served from the API's site (`app.paccradle.com` next to `api.paccradle.com`), or the refresh cookie is dropped.
    - State uses React built-ins only: `useReducer` and Context (DEC-05).
- [ ] **E2 · Capture flow** · _claude_ · §3 steps 1–3, §7.1
    - Choose the mode: photo, trace or type sizes.
    - Paper defaults by country.
    - Guidance screen (§5.1).
    - Upload: read EXIF with `exifr`, convert HEIC to JPEG on a canvas, resize to ≤ 4000 px, send the EXIF values as separate fields.
    - Only Safari/WebKit can draw HEIC on a canvas. Other desktop browsers get a "please upload a JPEG" message for HEIC files.
    - From C4: send `focal_length_35mm` from exifr's `FocalLengthIn35mmFormat`, only when the photo has it and it's above 0. It places the camera for the tall-product warning.
- [ ] **E3 · Detection review** · _claude_ · §3 step 4
    - Paper and product outline over the photo, size and confidence, draggable points, tap to pick the product.
    - From C3: draw `paper.corners_px` and each product's `contour_px` straight over the photo as uploaded (photo pixels). Send taps and dragged points back in that frame. Keep drags on the paper, since points off it are refused. For `contour_source: "user"`, show "your outline" instead of a confidence.
- [ ] **E4 · Design choices** · _claude_ · §3 steps 5–8
    - Height, quantity, insert style cards, material, box.
    - From A7: default a card's or cradle's margin to at least the board's web (3 mm or 3× the thickness: 12 mm for C-flute), or the hole-to-edge warning fires on a fresh design.
    - From C4: send the typed height to the capture (`set_height`, through D5) and show its `outline_may_be_bigger` warning: "up to X mm past the edge" next to a Trace mode button. Don't round the photo's sizes down by it: it's a bound, not a correction (that's M3-03).
- [ ] **E5 · 2D SVG editor (desktop)** · _claude_ · §7.1
    - Cut lines solid, creases dashed, dimensions shown.
    - Drag products inside the insert, undo/redo.
    - Live preview from the WASM build.
    - From A10: `loadInsertCore()` in `features/design/insert-core.ts`; `samples.ts` and `svg.ts` are a start. Run `generate` + `fits` once per animation frame and skip frames while one is still running. A card full of notched holes takes about 10–45 ms per frame. Preview paths are straight lines (within 0.05 mm of the true line, points to 0.01 mm).
- [ ] **E6 · 3D fold preview** · _claude_ · §7.2
    - From A4: `Insert.mounts` gives each cut copy's stance (flat, upright along x, upright along y) and position. Grid strips slide down to their mount.
    - From A6: `OuterBox.mounts` places the box in the insert's frame. The shipping box's root (its back wall) stands upright.
        - The `Stance` docs say which way each drawing's face looks: up for flat, +y for upright along x, −x for upright along y.
        - The engine's frame is a top view (x right, y down the page, z up), which is left-handed. three.js is right-handed with y up, so map (x, y, z) to (x, z, y).
        - A hinge's `offset_mm` is a bend. The strip of the child within `offset_mm` of the crease stays in the parent's plane (draw it with the parent's transform), and the rest of the child turns about the line `offset_mm` past the crease: child = parent × turn(line, fold_deg × progress). The strip and the line stay put as the fold runs, so nothing opens up at any point of the animation.
        - `tests/common/fold.rs` is the reference for the fold.
    - Built with `three` + `@react-three/fiber`. Orbit controls (smooth) and the orthographic camera come from `three/addons`, not drei (DEC-05).
    - Hinge tree, panels extruded by board thickness, staged folding, fold slider.
    - Isometric camera, orbit view, exploded view.
    - Partition strips slide into each other's slots.
    - retry mechanism if error to render.
    - show loader while waiting the model render.
- [ ] **E7 · Fit check** · _claude_ · §7.3
    - Colors in the 2D editor. Product prisms in 3D if there's time.
    - From A8: `Packaging::fits(design)` gives each copy's `color` (green, yellow, red), `clearance_mm`, `target_mm` and `clashes`. Explain yellow as "tighter than the X mm we suggest for this board" and red by its clashes.
    - From A10: the engine's `fits` call is the costly part of a frame (5–7 ms for 6 notched holes in the browser). If dragging feels slow, speed it up first (Parking lot).
- [ ] **E8 · Warnings panel** (from A7) · _claude_
    - From A7: `Packaging::warnings((design, options))` returns `{ rule: { kind, …numbers }, at: { part, piece, area_mm, placements } }`. Translate by `rule.kind` with `t()`; `Rule`'s `Display` is the English fallback.
    - Highlight `area_mm` on the piece's flat drawing (piece coordinates), and the products in `placements`. A warning with no `piece` is about the board (thickness).
- [ ] **E9 · Export + checkout** · _claude_ · §9.3
    - Free: watermarked PNG and test PDF (make watermark to not let the AI can not delete it if user want to take the photo and then they use ai to generate what we serve).
    - From B4: the test PDF is made on the server and its watermark is done (see B4). Default the paper by country, as in §3 step 2 (Letter for the US, Canada, Mexico and the Philippines; A4 elsewhere), and let the user switch.
    - From B5: the watermarked PNG is made on the server too (see B5). The download button only needs the export endpoint.
    - Paid: Paddle.js overlay checkout, wait for the unlock (SSE or polling `/me`), then download.
- [ ] **E10 · My designs**: list and reopen · _claude_
- [ ] **E11 · Phone layout**: capture and view only · _claude_ · §7.1

### M1-F · Billing (§9)

- [ ] **F1 · Paddle sandbox catalog** · _both_ · §9.2, DEC-02
    - Create the products and prices. Price IDs live in config.
- [ ] **F2 · `BillingProvider` trait + Paddle implementation** · _claude_ · §9.4
- [ ] **F3 · Webhook handler** · _claude_ · §9.4
    - `POST /api/webhooks/v1/paddle` (D1): its own surface, no session, no CORS. Use the raw body. HMAC-SHA256 over `ts:body`, compared in constant time.
    - Reject old timestamps. Idempotent through `paddle_events`.
    - Return 200 fast and do the work in a job.
- [ ] **F4 · Event handling** · _claude_ · §9.4
    - From D2: F3 stores the event (`ON CONFLICT (event_id) DO NOTHING`) and enqueues its processing job in one transaction. Skip a subscription event older than `subscriptions.paddle_updated_at`. A pack's `purchase` row is unique per transaction, so a replayed event can't double the credits.
    - `transaction.completed`: unlock the design or add credits.
    - `subscription.*`: update the subscription.
    - Refunds and chargebacks: reverse the unlock or credits.
- [ ] **F5 · Entitlements** · _claude_ · §9.4
    - Credit ledger. Unlocks allow edits for 30 days. Pro check.
    - Done when: there's a unit test for every branch in §9.3.
- [ ] **F6 · Customer portal link + daily repair job** · _claude_ · §9.4
- [ ] **F7 · Sandbox end-to-end test** · _both_
    - Use Paddle test cards: every product, a refund, a subscription cancel.
- [ ] **F8 · Go live** · _both_
    - Live keys and live webhook secret.
    - You make one real purchase and then refund it.
    - Needs Paddle approval (M0-06).

### M1-G · Production readiness

- [ ] **G1 · Deploy** · _claude_ · §8.2
    - Docker images, with the OpenCV runtime in the API image. VPS deploy, HTTPS, database backups.
    - From C1: Debian's `libopencv-imgcodecs410` pulls in GDAL, OpenEXR and TIFF, which the API never uses. A slimmer OpenCV build (core, imgproc, imgcodecs with JPEG, PNG and WebP only) would shrink the runtime image a lot.
- [ ] **G2 · Monitoring** · _claude_ · §13
    - Error tracking. Pick a tool. An SDK such as Sentry's is a new runtime dependency, so write down why (DEC-05).
    - Funnel events: visit → capture → design finished → export paid.
- [ ] **G3 · Final legal pages** · _both_ · §14
    - Files are design aids. Always test-print. No claims about shipping protection.
- [ ] **G4 · Account and photo deletion** · _claude_ · §8.5
    - From D2: deleting the `users` row deletes all their rows (cascade). Delete their R2 photos and export files first (a job), since the database can't.
- [ ] **G5 · Test on 3 phones** · _you_
    - An iPhone, a Samsung and a cheap Android. Every bug goes in a list.

### Gate 1

| Check                                                                                   | Target | Result |
| --------------------------------------------------------------------------------------- | ------ | ------ |
| End-to-end on 3 phones: capture → design → live checkout → download                     | works  |        |
| Open payment bugs                                                                       | 0      |        |
| Accuracy regression (C5)                                                                | green  |        |

---

## M2: Beta & launch (weeks 9–10, see DEC-10)

- [ ] **M2-01 · Invite 30 beta users** from the waitlist · _you_ · §12
    - Brands and designers first (§2). Send an onboarding email with a feedback form.
- [ ] **M2-02 · Fix the top 10 issues** · _claude_
    - Rank the feedback first, then fix the 10 most important problems.
- [ ] **M2-03 · SEO template pages** · _claude_ · §11.1
    - A generator that turns one spec into one page: live 3D preview, free test PDF, and a "make it fit **your** product" button.
    - 30 pages first. No brand names in titles.
- [ ] **M2-04 · How-to guides** · _both_ · §11.2
    - 4 guides: how to make a box insert, insert vs. partition, E-flute vs. B-flute, getting inserts made by a printer.
- [ ] **M2-05 · View-only 3D share link** · _claude_ · §3 step 11, §11.5
    - Optional before launch, but it's a cheap way to bring in new users.
- [ ] **M2-06 · Launch** · _you_ · §11.6
    - Product Hunt, communities, AI tool directories.
    - Before it: freeze the client API at `1.0.0` (drop `-beta.N`, D1). From then on a breaking change needs `/api/client/v2`.
    - Show HN about the Rust/WASM/vision engineering. I'll draft the write-up.

### Gate 2

| Check                                        | Target | Result |
| -------------------------------------------- | ------ | ------ |
| First paying customers (not friends)         | ≥ 1    |        |

---

## M3: v1 (weeks 11–16)

- [ ] **M3-01 · L2 SAM 2** via `Candle` (pin the exact version) · _claude_ · §5.4
    - Box prompt from L1. Route by confidence. Measure CPU time on the VPS.
- [ ] **M3-02 · L3 AI vision helper** · _claude_ · §5.4, DEC-13
    - One `VisionHelper` trait with three providers, tried in order: Hugging Face → Gemini free tier → fal.ai. Move on after a failure, a timeout (`AI_STEP_TIMEOUT_SECONDS`) or a quota error (HTTP 429).
    - Skip the Gemini free tier for visitors in the EEA, the UK and Switzerland, and when the country is unknown.
    - Send only the flattened paper image (≈1000 px). One adapter turns every provider's box format into paper pixels, with a test per provider.
    - JSON answers validated against a schema; blocked or empty answers count as failures. User opt-out.
    - Log each call's token usage and cost. Run all three on the M0-07 photo set and record the results here.
    - Retry rule from §5.4: retries 1–2 use L1/L2 only; retry 3 adds L3.
    - Check the API details against the current Hugging Face, Gemini and fal.ai docs when building it. The plan's notes are dated 2026-10-28.
- [ ] **M3-03 · Lay-down photo + pose correction** · _claude_ · U7, §5.2 fix C
    - Re-run the tall-item accuracy test (§5.6 row 3).
- [ ] **M3-04 · Live camera guide** · _claude_ · U6
    - Paper detected live in the browser with a green frame, plus a level indicator from the gyroscope.
- [ ] **M3-05 · QR handoff from desktop to phone** · _claude_ · §3 step 1d, §8.3
- [ ] **M3-06 · Kit builder** · _claude_ · U5
    - Several products → automatic layout → one insert + one box.
- [ ] **M3-07 · Maker presets** · _claude_ · U8
    - Cricut/Silhouette SVG with a score layer, 12×12″ and 12×24″ mats, kerf offset.
- [ ] **M3-08 · Unboxing video** · _claude_ · U9
    - `captureStream` + `MediaRecorder`, watermark on the free plan.
- [ ] **M3-09 · Printer spec sheet PDF** · _claude_ · U10
- [ ] **M3-10 · Empty-space ratio + board area/weight**, labelled "estimate" · _claude_ · U11, §6.3
- [ ] **M3-11 · AI insert advisor** · _claude_ · U12
- [ ] **M3-12 · Bottle shapes + 3D fit colors** · _claude_ · §7.3
    - Lathe the side-photo profile into a bottle stand-in, then color the fit in 3D.
- [ ] **M3-13 · GLB export + isometric PNG renders** · _claude_ · §6.6
- [ ] **M3-14 · More SEO pages** toward 100+ · _both_ · §11.1

### Gate 3 (§13)

| Metric                                         | Target  | Result |
| ---------------------------------------------- | ------- | ------ |
| Visitors who upload a photo or type sizes      | ≥ 15%   |        |
| Detections that need no manual fix             | ≥ 80%   |        |
| Started designs that are finished              | ≥ 50%   |        |
| Finished designs that are paid for             | ≥ 3–5%  |        |

---

## M4: v2 (month 5+)

- [ ] **M4-01 · CF2 export + PDF with named spot colors** · U13
- [ ] **M4-02 · Studio plan** · §10
    - 3 seats, brand library, priority support, and a Paddle product for it.
- [ ] **M4-03 · Pocket tray + neck holder** · §6.2, DEC-12
- [ ] **M4-04 · Layered cardboard nest** · U15
- [ ] **M4-05 · More languages**: Bahasa Indonesia, Spanish, Portuguese · DEC-11
- [ ] **M4-06 · Printer widget + "for printers" page** · U14

### Gate 4

Studio demand from inbound requests (§12).

---

## Recurring

- **Weekly:** update Current focus and the Progress log; look at the funnel numbers (§13).
- **Monthly:** re-check the dated facts in CLAUDE.md: Paddle fees, Pacdora features, model prices, crate versions (`opencv`, `Caddle`).
- **3 months after launch:** apply the revenue rule (§13):
    - ≥ $1,000/month → keep going.
    - < $300/month → change focus.
- **Every price change:** keep every Paddle checkout at $10 or more (§9.1). The $10 single export is already at that line, so no coupons on it.

---

## Parking lot

Ideas that came up mid-build. Review them at each gate.

- **Inch display** for US and other Letter-paper visitors (the site is global). The engine stays in mm; the editor converts (E4/E5).
- **Closed cradle ends**: the cradle's short ends are open (A5). If beta users want them closed for looks, add end walls that tuck into the leg tubes.
- **Cradle legs along the longer side**: legs always run along x, even when the deck comes out wider than long. Choose the longer side if decks sag.
- **Clearer "doesn't fit" sizes** (A3): `DoesNotFit` reports the spread of the holes. When a placement runs off the card, that can come out smaller than the room ("needs 112 × 124 mm, box leaves 146 × 146 mm"). Measure from the card's corner instead.
- **Slots at least 1 mm wide in paperboard** (A7): every slotted paperboard design warns, because its slots are 0.5 + 0.3 = 0.8 mm. Cutting them `max(t + tolerance, 1 mm)` wide stays inside the slot-fit band (0.5 mm of play). The paperboard mailer's 0.55 mm slit between dust flap and end wall would need its own fix. Paperboard is unusual for grids and boxes; decide when a beta user wants it.
- **Product angle in auto layouts** (A9): the layout turns each product to its smallest bounding box. That turns the pump bottle sample about 25° (about 4% less board). Keeping the angle the product was measured at may look more natural. Decide when beta users see it.
- **Faster fit check** (A10): `fits` rebuilds every product's hole with the generator's code. That's most of a slider frame for cards and cradles: 5–7 ms of the 10 ms for 6 notched holes in the browser, about 35 ms for 24. Keeping the holes from `generate`, or one `preview` call returning both, would roughly halve it. Do it in E5/E7 if dragging feels slow.
- **Every copy laid out for makers** (B1): production files draw each piece once. When the maker presets come (U8), lay out every copy on the chosen 12×12″ or 12×24″ mat instead.
- **Lid ears on the mailer**: the lid closes with its front tuck flap only. Many roll-end tuck-front mailers also have small flaps on the lid's sides that tuck into the ends. Add them if beta users want a firmer close.
- **Bigger paper for the test print** (B4): a shipping box takes 8 Letter tiles. An A3 / Tabloid option would halve that for people with such a printer.
- **Test print text in other languages** (B4): the instructions are English strings in `export/test_print/instructions.rs`. Move them to translatable tables with the first new language (DEC-11, M4-05).
- **The name in files** (B4, B5): the watermarks ("PACCRADLE TEST PRINT", "PACCRADLE PREVIEW"), the PDF titles and the PNG's footer use the working name. Change them with DEC-01.
- **Smaller PNG previews** (B5): tiny-skia always writes full-colour PNGs with transparency, about 590 KB for the samples. Writing RGB without transparency, or fewer colours, would roughly halve that if storage or download size starts to matter.
- **Share-link and SEO images from the PNG code** (B5): `Preview` and its drawing code can make Open Graph images for share links (M2-05), template-page thumbnails (M2-03) and, later, the isometric renders (M3-13).
- **Which size to trust for detected outlines** (C3): `bbox_mm` and `diameter_mm` come from every edge pixel plus half a pixel, while `contour_mm` (what `insert_core` cuts round) keeps fewer points (ε 0.2 mm) and runs up to about 0.25 mm smaller. On the synthetic 80 × 30 box the outline was closer to the truth (80.08 × 30.02 vs 80.25 × 30.26). Check both against the calipers in M0-11, and drop or shrink the half-pixel allowance if the outline wins on real photos too.
- **A tap that finds what the threshold missed** (C3): a tap only picks among the objects the colour threshold already found. For white-on-white products, a tap could seed a weaker threshold or GrabCut round the tap instead. Tune it on the white and glass items in M0-07.
- **Read the focal length on the server** (C4): photos sent straight from a phone keep their EXIF, but only the browser reads it (E2), so `measure` and the accuracy report assume a 24 mm lens. A small EXIF reader for `FocalLengthIn35mmFilm` (next to the header checks in `image.rs`) would place the camera better there, and for any upload where the browser sent nothing.
- **Tell the user to hold the phone level** (C4): `camera.foot_mm` shows how far off the sheet the phone was. A tilted phone makes tall products much worse (51.9 mm instead of 7.8 mm in the tests); a "hold the phone straight above" hint when the foot lies far from the product would help, as would the live level guide (M3-04).
- **Drag the paper corners** (C3): when the paper itself is found wrong, the scale is wrong and no tap or edit fixes it. Letting E3 move the 4 corners would re-run detection with them.
- **Tidy the backend env examples**: they still mention a PWA and "progress photos" from another project. The RepVane passkey lines are gone (D3). The webhook path is settled: `/api/webhooks/v1/paddle` (D1).
- **Back-office endpoints** (D1 made the admin surface): find a user, grant or take back credits, refund help, retry dead-letter jobs, review low-confidence captures for the accuracy set. Pick the first ones after D3, when admins can sign in.
- **Wake workers at once** (D2): an idle worker polls every 500 ms, which adds up to half a second to a vision result. Postgres `LISTEN/NOTIFY` (sqlx's `PgListener`) after the enqueuing transaction commits would wake one at once.
- **Delete old guests** (D2): guest rows with nothing saved could go after 30 days, by a job.
- **Admin surface behind an IP allowlist** (D1): Nginx could allow `/api/admin/` only from your addresses, on top of admin sign-in.
- **List, rename and delete passkeys** (D3): `GET /me/passkeys` (paginated, your rule), `PATCH` a name, `DELETE` one. Wanted before the beta, so a lost device's passkey can be removed.
- **Breached-password check** (D3): ask Have I Been Pwned's k-anonymity range API when a password is set (only the first 5 characters of its SHA-1 leave the server).
- **Scheduled sign-in housekeeping** (D3): expired sessions, links and challenges are pruned when new ones are made. A daily job (with D4's daily enqueue) would keep the tables small on quiet days.
- **Change email, sign out everywhere, HTML emails** (D3): a link to the new address; revoke every session; a simple HTML part next to the text.
- **Admin roles from the back office** (D3): granting the role is SQL for now. A "make admin" route would need two admins or a CLI.
- **Development web app on a paccradle.com subdomain** (D3): the refresh cookie is `SameSite=Strict` and browsers block cookies across sites, so `paccradle-app.netlify.app` against `api-dev.paccradle.com` can't keep a session. Point a domain such as `app-dev.paccradle.com` at Netlify, and set `WEBAUTHN_RP_ID` to match.

---

## Decision log

| Date       | ID     | Decision                                                      | Why                                                                         |
| ---------- | ------ | ------------------------------------------------------------- | --------------------------------------------------------------------------- |
| 2026-10-27 | DEC-04 | Frontend uses **Next.js** (not Vite)                          | Your choice. Already scaffolded; gives static HTML for the SEO pages (§11.1). |
| 2026-10-27 | DEC-05 | **No Tailwind**: vanilla CSS with CSS Modules + design tokens | Your choice. Matches your frontend rules. Which dependencies to allow is still open. |
| 2026-10-27 | DEC-02 | **$10 export / $35 5-pack / $12 Pro per month / $139 Pro per year; Studio $49** | Your choice: the lowest prices that keep every checkout at Paddle's $10 line. |
| 2026-10-27 | DEC-03 | **Monorepo at `pac-cradle/`; `insert_core` and `vision` are crates inside the `paccradle-backend` Cargo workspace** | Your choice. Plan and code are in one place, there's one CI, and the frontend imports the WASM by local path. |
| 2026-10-27 | DEC-05 | **Runtime dependencies:** `three` + `@react-three/fiber` (no drei), React built-ins for state (no Zustand/TanStack), `exifr`, Plausible script, plus Paddle.js and the `insert_core` WASM | Your choice. Keeps your vanilla-first rule; each library replaces code that is hard to write by hand. |
| 2026-10-27 | M0-02  | API contract = the dated Postman files; frontend types are written by hand from them (no utoipa / openapi-typescript) | My pick: your backend code-style rule asks for Postman files, and one source can't drift from a second. Change it if you prefer generated types. |
| 2026-10-27 | M0-10  | Trace mode adds back **half** of `--pen-width-mm` (0.5 mm pen → 0.25 mm per side) | My reading of §5.2 fix A; calibrate it with the M0-11 report. |
| 2026-10-28 | DEC-13 | **Backup AI: Hugging Face `Qwen3-VL-8B-Instruct` → Gemini free tier → fal.ai Moondream 3.** No Claude API. Gemini's free tier is skipped for visitors in the EEA, UK and Switzerland | Your chain, with vision-language models in place of SD line art and ControlNet (those draw pictures; they can't find a product). Checked: free HF accounts get no monthly credits; Google allows only paid Gemini for apps serving EEA/UK/Swiss users, wherever the developer is. |
| 2026-10-28 | Gate 0 | **Started M1 before Gate 0 passed** | Your request. The open M0 tasks wait on you (photos, Paddle, domain), and M1-A doesn't depend on their results. Risk: building before demand is proven. Gate 0 still decides go / adjust / stop. |
| 2026-10-28 | M1-A1 | Design JSON uses its own plain geometry types (points as `[x, y]`, y pointing down like the vision output; arcs as centre + sweep), not `kurbo`'s serde format | Stored designs must not change when a library does. A2 converts to `kurbo` inside the crate. |
| 2026-10-28 | M1-A1 | `InsertSpec` has only the 3 MVP styles (DEC-08 suggestion); the outer box has its own board and an optional fixed inside size | Scope lock (§14). A brand's box is often corrugated while the insert is paperboard, and some brands already stock a box. |
| 2026-10-28 | M1-A1 | serde_json `float_roundtrip` feature on for the whole workspace | The round-trip test found values coming back 1 bit off. Saved designs must read back exactly. |
| 2026-10-28 | Design | **Visual direction:** sustainable-packaging palette (oat, cream, kraft, terracotta, sage, espresso) from your two reference images; Fraunces + Geist; "ultra hi-fi" motion (text, scroll, hover, page transitions) built with CSS and one small client controller, no animation library | Your request and references; keeps DEC-05 (no new runtime dependency). Details in the frontend rules. |
| 2026-10-28 | M1-A2 | **`i_overlay` 9.1** for offsets and booleans, not Clipper2 | Spike on the A2 tests: `i_overlay` and the pure-Rust port `clipper2-rust` both landed at 0.002 mm and built for WASM. The C++ `clipper2` binding fails to build for WASM. `i_overlay` won on maturity: it's the engine inside `geo`, with about 10.8M downloads vs. 170K for a port first published in Feb 2026. Its output also groups holes with their region, like our `Panel`. Cost: about 39 KB more gzipped WASM (87 vs. 48 KB in the spike). |
| 2026-10-28 | M1-A2 | **No `kurbo`**; arcs are flattened by our own code | Our paths only have lines and circular arcs, and flattening an arc is a few lines of exact math. `kurbo` would first turn arcs into Bézier curves, which adds error and a dependency (your rule: no crate when plain Rust can do it). Reversible: add it if Bézier curves are ever needed. |
| 2026-10-28 | M1-A2 | Curves use a **0.002 mm** chord tolerance; the polygon library snaps to a **0.1 µm** grid | Keeps offsets 5× inside the 0.01 mm bar, far finer than any cutter. A fixed grid gives the same results natively and in WASM. Everything in one operation must fit in about 100 m across. |
| 2026-10-28 | M1-A3 | **Generators return an `Insert`**: pieces, footprint and height, and a placement for every product copy | The box (A6) needs the insert's size, and the fit check (A8) and 3D stand-ins need where each product sits. The editor sends the placements back to keep the layout while the user drags. |
| 2026-10-28 | M1-A3 | A placement's centre is the **centre of the outline's bounding box, taken before turning** | The A1 docs only said "centre of the outline". The bounding box matches the vision output's `bbox_mm` and is easy to explain. Doc updated; the JSON is unchanged. |
| 2026-10-28 | M1-A3 | Auto layout **straightens** each product (smallest enclosing rectangle, long side along x) and packs rows that keep the card close to square | Vision keeps outlines at the angle they were photographed. Without this, a box shot at 30° wastes card space. |
| 2026-10-28 | M1-A3 | **Finger notch**: one half-round notch, 19 mm wide, halfway along a long side of each hole; not a setting yet | A fingertip width (¾ inch is a common packaging size). One side keeps the card stronger than two. Make it a setting if beta users ask. |
| 2026-10-28 | M1-A3 | Fixed box: card = inside size − **the box's** clearance on each side | §6.2 says "box inside size − clearance". The gap is between the card and the box walls, so the box's own clearance applies. |
| 2026-10-28 | M1-A3 | Overlapping products **share a merged hole** instead of failing; A8 shows them red | While the user drags in the editor, a failing preview would go blank. Only a card that falls apart, or rows that don't fit a fixed box, are errors. |
| 2026-10-28 | M1-A3 | **Finger notch side and position are settings** (replaces "not a setting yet" above): `side` bottom/top/both and `along` 0–1, default one notch in the middle of the bottom long side. The size stays fixed at 19 mm | Your request: users may want the notch elsewhere. |
| 2026-10-28 | M1-A3 | **`align` setting** for the generator's rows in a fixed box, 9 positions, default centre | Your request: the centred default can be changed. Exact positions still come from `placements`. |
| 2026-10-28 | M1-A3 | JSON change: the card's `finger_notches` went from `true`/`false` to `null` or `{ side, along }`; both new fields can be left out | Nothing is stored yet (no database, no frontend), so the clean shape wins over a bool kept for compatibility. |
| 2026-10-28 | M1-A4 | **`Insert.mounts`**: one entry per cut copy, with the piece, its stance (flat / upright along x / upright along y) and position in mm | The 3D preview needs to know where each grid strip stands, and the line-up test checks against the same data. Cards get one flat mount. |
| 2026-10-28 | M1-A4 | **Grid strips**: long dividers slotted from the bottom, short ones from the top, both half the height + 0.5 mm deep; with a frame, strips run half a slot + one web past the corners | Strips slide together with tops and bottoms level and 1 mm to spare (§6.2). The end tab keeps the §6.5 web of board beside the corner slots. |
| 2026-10-28 | M1-A4 | In a **fixed box the grid stretches** its cells to fill it (and tries products crosswise before giving up) | A grid has to touch the box walls or it slides around. |
| 2026-10-28 | M1-A4 | Grid **finger notches stay on/off**: one centred notch per cell wall; no position setting | A strip's top edge has only one sensible place per wall. Add `along` like the card's if you want it. |
| 2026-10-28 | M1-A4 | `DoesNotFit` fields renamed `room_length_mm` / `room_width_mm` (were `card_…`) | The grid uses the same error, and nothing is stored yet. |
| 2026-10-28 | M1-A5 | **Cradle = deck on two folded tube legs** along the long edges. Each leg is wall → foot → riser, with lock tabs through slots in the deck, flush with its top. The short ends stay open | One piece and no glue. Every fold is 90° in the same direction, so the 3D fold has no board sandwiched in a 180° fold. A design with four walls and corner flaps needs flaps trapped in double walls, which the middle-plane hinge model can't show. The box walls stop the deck sliding. |
| 2026-10-28 | M1-A5 | **Fold model: panels turn about creases on the board's middle plane**; flat lengths between creases are middle-plane lengths. A flat mount's z is the piece's underside | One rule for the generators, the 3D preview (E6) and the fold tests. Written in the `piece` module docs. |
| 2026-10-28 | M1-A5 | Leg and tab sizes come from the board and deck height, not settings: foot = half a slot + a web, tabs up to 15 mm, about one per 60 mm | The foot keeps a full web between each lock slot and the deck's crease (§6.5). Make them settings if beta users ask. |
| 2026-10-28 | M1-A5 | A product no taller than the deck is **an A7 warning, not an error** | Dragging the deck-height slider would otherwise blank the preview, the same reasoning as merged holes in A3. |
| 2026-10-28 | M1-A5 | JSON change: `align` added to the cradle settings (can be left out; default centre). The fixture now lists it | Same setting as the card. Nothing is stored yet. |
| 2026-10-29 | M1-A6 | **`Design::generate()` returns a `Packaging`**: the insert and, if the design has one, its outer box. `generate_insert()` stays | One call for the editor and the exports. The box needs the generated insert's size. |
| 2026-10-29 | M1-A6 | **Box size**: inside = insert + the box's clearance each side; height = tallest of insert and products + the clearance. A fixed inside size is kept: a taller product gives `TooTallForBox`, and a size below the style's smallest gives `BoxTooSmall`. A box sized round the insert grows to that smallest size | §6.3. The clearance is the gap between the insert and the box walls, so the box's own value applies (as in A3). |
| 2026-10-29 | M1-A6 | **Mailer = FEFCO 0427 style, one piece, no glue**: double end walls trap the dust flaps, lock tabs on the inner end walls go through base slots, and the lid closes with a front tuck flap (no lid ears). Tuck depth min(30 mm, half the height) | Every crease is a 90° valley fold, and the lock tabs reuse the cradle's. A brand can fold it without glue or tape. |
| 2026-10-29 | M1-A6 | **Shipping box = FEFCO 0201** with the joint flap outside the back wall. The longer walls carry the major flaps; every flap is (shorter side + t − g) / 2 deep. The walls with major flaps are 2 t taller between creases, so major flaps lie flat over the minor ones | The inside stays clear for the insert. The flaps meet with the slot tolerance between them, and the bottom stays flat. The stepped scores are noted in the Parking lot. |
| 2026-10-29 | M1-A6 | **Box drawings face the inside**; exports mirror box pieces so the print side faces up | All folds are valley folds when drawn from the inside, the same as the 3D fold. Mirroring belongs in the exporters (B1–B4), not in the geometry. |
| 2026-10-29 | M1-A6 | **Fold tests treat folded board as solid blocks** (`tests/common/solid.rs`), one board thick. Panels joined by a hinge may overlap at their crease | Exact checks for collisions, a clear inside and closed sides, instead of sampling points. Works because every box and cradle fold is a quarter turn. |
| 2026-10-29 | M1-A6 | **Shipping box scores run straight** (replaces the stepped creases above): all walls the same height, all flaps the same depth. The outer flaps turn a board past their crease | Your review: the corners and flap tips didn't line up. A real 0201 has straight scores, and a box plant's slotter can only score straight. The outer flap's bend rolls round the inner flaps; the bend's own length isn't modelled, so test cuts tune the flap depth. |
| 2026-10-29 | M1-A6 | JSON change: **`Hinge.offset_mm`**, how far past its crease a panel turns (0 for most creases) | Keeps the die line straight while the 3D fold stays free of collisions. One number per hinge; E6 grows it with the fold. Nothing is stored yet. |
| 2026-10-29 | M1-A6 | **`Hinge.offset_mm` is a bend** (replaces "carried past the crease" above): the child's strip next to the crease stays in the parent's plane, and the rest turns about the line `offset_mm` past the crease. Outer flaps are cut a board deeper to pay for it; inner flaps match, except in nearly square boxes | Your second review: carried past the crease with nothing in between, the outer flaps floated half a board below their walls. Modelled as a bend, wall and flap are one board, the outer flaps still meet, and the drawn flap length matches the board the 3D uses. |
| 2026-10-29 | M1-A7 | **Warnings are their own call**: `Packaging::warnings((design, CheckOptions))` returns `Warning { rule, at }`. `rule` has a `kind` and its numbers; `at` names insert or box, the piece, the feature's box in the piece's drawing, and the product copies | They cost more than a preview frame (up to about 30 ms), and the editor needs to point at the piece, the spot and the products. Text comes from `t()` by `kind`; `Display` is the English fallback. |
| 2026-10-29 | M1-A7 | **Hole rules use each product copy's own hole**, rebuilt with the generator's code, not the cut result. Holes that hold no product are lock slots, and the crease rule skips them | The cut merges overlapping products into one hole and loses products placed off the card. Skipping lock slots covers both exemptions noted in A5 and A6 with one rule. |
| 2026-10-29 | M1-A7 | **"Smaller than 1 mm" means board or cut-outs narrower than 1 mm** (shrink and regrow the outline and each hole by 0.5 mm). Shallow steps don't count. Thin board between holes, or between a hole and the edge, is reported by the spacing rules | That's what tears or can't be cut. A step, like a 0.5 mm paperboard tab on a wide panel, cuts fine. The spacing rules' limit (at least 3 mm) is stricter, so nothing is lost. |
| 2026-10-29 | M1-A7 | **Slot fit = slot tolerance outside 0.1–1 mm** (estimate), only for slots that take board: grid strips, cradle deck, mailer base | Slots are cut a board plus the tolerance wide, so the tolerance is the only thing that can be off. The band brackets the §6.4 starting values (0.3 and 0.5 mm). |
| 2026-10-29 | M1-A7 | **The sheet is an option** (`CheckOptions.sheet`), with no presets yet | A design has no sheet: it's chosen at export (B tasks, maker presets U8). |
| 2026-10-29 | M1-A7 | Over a leg = the product's box reaches into the strip under a long edge of the deck, in to the riser's inner face. No taller than the deck = height ≤ deck height | The leg runs the deck's full length, so the box test is exact. These are the A5 notes. |
| 2026-10-29 | M1-A8 | **Fit target = the clearance recommended for the insert's board** (§6.4), not the design's own clearance | The generator cuts every hole at the design's clearance, so measuring against that would always say green. Against the recommendation, a user who dials the clearance down for a snug fit sees yellow, which tells them something. |
| 2026-10-29 | M1-A8 | **The fit looks at each product from above**: flat pieces' root panels as boards with holes, upright pieces as strips one board thick, the cradle's legs as strips under the deck's long edges, the insert's footprint as the edge | One method for all three styles, built from data the insert already has (mounts, panels), with no folding in the engine. |
| 2026-10-29 | M1-A8 | **Red = any clash, including touching within 0.01 mm**. Products that overlap each other are red, even though their merged hole clears the board | Touching is no clearance at the accuracy we cut to. The A3 note asked for overlapping products to show red. |
| 2026-10-29 | M1-A10 | **Browser bindings in a wrapper crate, `insert_core_wasm`** (not a feature of `insert_core`) | `wasm-bindgen`'s glue needs `unsafe`, which the workspace forbids, and a `cdylib` `insert_core` would slow every native build. The wrapper never enables the export writers. CLAUDE.md §6.1 notes it. |
| 2026-10-29 | M1-A10 | **Preview lines: straight segments within 0.05 mm, points to 0.01 mm** (was a 0.5 mm grid) | Your choice after review: the 0.5 mm grid made curves stair-stepped and copies of one hole different. Protection rests on no arcs, no layers, server-only production files, payment and watermarks. CLAUDE.md §7.4 reworded. |
| 2026-10-29 | M1-B1 | **Production files draw each piece once**, with how many to cut in its title | A die maker builds one die per piece; cutter apps duplicate a shape in a click. Laying out every copy on mats is the maker presets' job (U8, v1). |
| 2026-10-29 | M1-B1 | **Layers CUT `#ff0000`, CREASE `#0000ff`, PERFORATION `#00a000`; 0.025 mm hairlines; no drawn text in SVG** | The common laser convention (red cut, blue score); hairlines so Epilog-style drivers cut rather than engrave; text in a cut file gets cut by Cricut. Labels belong in the PDF (B2). |
| 2026-10-29 | M1-B1 | **SVG written by hand**, without the `svg` crate (CLAUDE.md §6.1 suggested it) | It's plain text (your rule: no crate when vanilla Rust is enough). |
| 2026-10-29 | M1-B2 | **PDF written by hand**, not `printpdf` or `krilla` (as the task suggested) | Lines, colours, layers and text in a built-in font are a small part of PDF. `krilla` would need a font file embedded for the labels; `printpdf` brings many crates. Your rule: no crate when vanilla Rust is enough. CLAUDE.md §6.1 updated. |
| 2026-10-29 | M1-B2 | **The PDF draws creases dashed as well as blue**, with labels and dimensions on their own layers; pieces 25 mm apart (`Spacing::DRAWING`) | The PDF is read by people and printers: dashed creases are the usual dieline look (§3), and die makers turn off the LABELS and DIMENSIONS layers. Cut files stay solid, without text, 10 mm apart. |
| 2026-10-29 | M1-B3 | **DXF R12, written by hand**, not the `dxf` crate (CLAUDE.md §6.1 suggested it) | R12 is the DXF nearly every CAD, laser and cutter program reads, and it's plain text: pairs of lines, a code then a value. Your rule: no crate when vanilla Rust is enough. CLAUDE.md §6.1 updated. |
| 2026-10-29 | M1-B3 | **One closed `POLYLINE` per path, arcs as bulges; no text** | A cutter follows a polyline in one go, where loose `LINE`s get cut in any order. Bulges keep arcs exact inside the polyline. Text in a cut file gets cut; quantities are in the PDF. |
| 2026-10-29 | M1-B4 | **The test print prints every copy of every piece**: whole when it fits a page (turned if that fits better), otherwise tiled with a 10 mm overlap; tiles the piece doesn't reach are left out | A free paper prototype should build in one go without thinking. Small pieces never get split, and big ones take the fewest pages. |
| 2026-10-29 | M1-B4 | **The test print is preview-grade**: the `rounded_for_preview()` lines, no layers, each tile's lines cut to its page in Rust, and a watermark over the lines on every page | It's free, so it may hold nothing more than the browser preview already does (§7.4). No page holds a whole tiled piece, and removing the watermark means redrawing lines. |
| 2026-10-29 | M1-B4 | **Seams: cut the hatched strip off a tile's left and top, lay that edge on the next tile's line; grey diamonds across each seam** | The sewing-pattern way: it works with opaque paper, because the diamond halves left after cutting only join up when the pages line up. |
| 2026-10-29 | M1-B4 | **Fold steps: each hinge stage is a numbered circle on its crease; white = away from the printed side, black = towards it** | The A6 note: box pieces print mirrored, so their folds go away from the printed side. A number on the crease needs no panel names, and the instructions list the same steps. |
| 2026-10-29 | M1-B4 | **Scale bar on every page (50 mm on A4, 2 in on Letter), and `/PrintScaling /None` in the file** | "Fit to page" shrinks a print by a few percent, enough to spoil a fit check. Letter users mostly have inch rulers. |
| 2026-10-29 | M1-B5 | **The PNG preview is drawn on the server with `tiny-skia` 0.12 and `ttf-parser` 0.25, with the Geist font bundled** (SIL Open Font License) | Your choice, over OpenCV (dated fonts, tests only in Docker), the browser canvas (no images for share links or emails) and drawing everything by hand. Smooth lines, PNG writing and letter shapes are where a crate is cleaner (your rule's exception). Server only, behind `export`. Cost: 17 compiled crates, mostly `png`'s compression stack. |
| 2026-10-29 | M1-B5 | **The PNG shows the sizes and fit in words, the top view coloured by fit, and every piece flat with numbered folds** | Your choice. The top view gets its own, larger scale (about 720 px long, labelled "enlarged"), because it's the picture a client looks at and the box's scale leaves it small. |
| 2026-10-29 | M1-B5 | **Golden PNGs are compared pixel by pixel within a tolerance** (at most 0.1 % of pixels off by more than 24 levels), not byte by byte | tiny-skia smooths with SIMD code that can differ by a shade between processors (this Mac, Docker, CI). The same machine still gives identical bytes, which a test checks. |
| 2026-10-29 | M1-C1 | **One entry point for capture jobs: `vision::run((CaptureInput, image bytes))`** returns the §5.5 result or an error with a stable `CAPTURE_*` code | The job (D5) stays a thin wrapper: no OpenCV types or file paths cross into the API. Plain computation, so it runs in a blocking task. |
| 2026-10-29 | M1-C1 | **Uploads are checked from the header before decoding**: JPEG, PNG or WebP only, at most 40 million pixels, at least 1000 px on the long side | §8.5 asks for a file-type check and a pixel cap against crafted files; reading the header costs nothing. 40 MP leaves room for photos sent straight from a phone (the browser sends at most 4000 px on the long side). Below about 1000 px a sheet gets under 3 px per mm, too coarse for the §5.6 targets. |
| 2026-10-29 | M1-C1 | **Photo warnings and errors are codes** (`kind` with its numbers; `CAPTURE_*`), with English as the fallback text | The editor translates every string (DEC-11), as with the A7 warnings. CLAUDE.md §5.5's example changed from sentences to `{ "kind": … }`. |
| 2026-10-29 | M1-C1 | **The photo's EXIF orientation turns it upright on decode; the EXIF values the browser sends are kept for later, not used to turn the photo** | The browser re-encodes photos already upright, so most arrive without EXIF. Turning them by the sent values as well would turn some twice. The sent focal length is for pose correction (M3-03). |
| 2026-10-29 | M1-C3 | **Taps and edited outlines come in photo pixels, and every detected outline goes out in photo pixels too (`contour_px`)** | A tap has to work when the first run found no product, and then there are no paper corners in a result to map with. The editor draws, drags and sends outlines without projection maths of its own; the mapping lives in one tested place (`PaperMapping`). CLAUDE.md §5.5 and §8.3 updated. |
| 2026-10-29 | M1-C3 | **A tap keeps the outline holding it, or the nearest within 10 mm. Other objects then don't count against it** | 10 mm is about a fingertip's miss on a phone showing the whole sheet. The user has said which object is the product, so the others shouldn't lower its confidence or warn. |
| 2026-10-29 | M1-C3 | **An edited outline is measured as drawn, from the stored result alone**: no photo, no simplifying, no half-pixel edge allowance; confidence 1, `contour_source: "user"`, only the parallax warning | "Accepted as it is". The user's points are the edge. The other warnings are about finding the product, which the user has now done. Parallax still applies, because they edit what the photo shows. |
| 2026-10-29 | M1-C3 | **Edited outlines that cross or touch themselves are refused** (`CAPTURE_OUTLINE_CROSSES_ITSELF`), as are points off the paper, under 25 mm², and fewer than 3 or more than 2,000 points | These points become cutting lines, and `insert_core` doesn't check for crossings. A crossing outline has no real area: a bow tie's halves cancel out. The cap keeps the check (every pair of edges) to a few milliseconds. |
| 2026-10-29 | M1-C4 | **Built for DEC-09's suggestion: the user types the height** (input field or `set_height`); DEC-09 stays open for your confirmation | M1-A was built for DEC-08's suggested styles the same way. The typed height is needed whichever way DEC-09 goes: the lay-down photo (M3-03) adds to it. |
| 2026-10-29 | M1-C4 | **The camera is worked out from the paper's corners and the lens** (EXIF focal length from the browser, else 24 mm assumed), and every result carries it (`camera`) | §5.2 fix B warns "when height ÷ camera distance is too big", so the warning needs the camera's height. The paper is a known rectangle, so its corners place the camera once the lens is known. 24 mm is wider than most phone lenses: the camera comes out nearer, and the warning errs on the safe side. Stored with the result, so `set_height` needs no photo. M3-03 can build on it. |
| 2026-10-29 | M1-C4 | **The tall-product warning gives a number: how far the outline may reach past the real edge, per side** (height ÷ camera height × the outline's farthest distance from below the lens); none under 0.5 mm | It's geometry, not a guess: exact for an upright cylinder, an upper bound for anything narrower at the top, and it covers tilted phones. Per side, like the clearance it eats into. 0.5 mm is about what §5.6 allows on each side for photo mode. |
| 2026-10-29 | M1-C4 | **The warning bounds the error; it doesn't correct the size** | Correcting needs to know where the product is widest, which a top view can't tell: that's M3-03's lay-down photo and pose correction. |
| 2026-10-29 | M1-D1 | **API surfaces: `/api/client/v1` (users), `/api/admin/v1` (back office), `/api/webhooks/v1` (Paddle); unversioned `/healthz`, `/readyz`** | Your request to separate clients from the back office with semantic versions. The surface comes before the version, so admin can change major version without the client. The name is singular, `client`, to pair with `admin` (your rule says `/clients`; it's one constant in `shared/api_surface.rs` if you want the plural). |
| 2026-10-29 | M1-D1 | **Semver per surface: major in the URL, full version in the `api-version` header; `1.0.0-beta.N` until launch** | Putting minor and patch in the URL would break callers on every fix. The pre-release tag is semver's own way to say "may still change" while only our web app calls the API. |
| 2026-10-29 | M1-D1 | **The admin surface fails closed: every admin path, unknown ones too, answers 401 until admin sign-in exists** | No back-office route can be reached by accident, and callers can't map which admin routes exist. |
| 2026-10-29 | M1-D1 | **Probes moved from `/api/v1/health` and `/api/v1/ready` to `/healthz` and `/readyz`** | They belong to no surface, and the static sites already use `/healthz`. Deploy script, CI smoke test and docs updated. |
| 2026-10-29 | M1-D1 | **The API compresses responses itself, except those that set a cookie** | D1 asked for compression; Nginx has `gzip off` because of BREACH. Every token-carrying response also sets the refresh cookie, so skipping those keeps tokens out of compressed bodies. |
| 2026-10-29 | M1-D1 | **No dotenvy: a small `.env` reader with Docker's rules, looked up after the real environment** | dotenvy writes the process environment, which is `unsafe` in edition 2024 (the workspace forbids it), and a reader with Docker's rules means one file works locally and in production. |
| 2026-10-29 | M1-D1 | **Own timeout and rate-limit middleware instead of tower-http's timeout and a governor crate** | tower-http's timeout answers with an empty body, not our error JSON; a per-IP token bucket is about 60 lines (vanilla-Rust rule), one process serves the site, and sign-in and uploads can reuse it with their own keys. |
| 2026-10-29 | M1-D2 | **Guests are users without an email** | Lets visitors capture and design before signing in, which the §13 "visitors who upload a photo" target needs, while every row still has an owner. Whether to require sign-in first stays open for D3; the schema works either way. |
| 2026-10-29 | M1-D2 | **IDs are made by the API (UUID v7), not by Postgres** | Postgres 18 has `uuidv7()`, but the managed database (DEC-06) may be older. |
| 2026-10-29 | M1-D2 | **Enum-like columns are checked text, not Postgres enum types** | A new value is a constraint change in one migration; the Rust side maps to enums either way. |
| 2026-10-29 | M1-D2 | **The credit ledger is append-only, enforced by a trigger** | The balance is `SUM(delta)`; an edited row would change history silently. Mistakes get an `adjustment` row with a note. |
| 2026-10-29 | M1-D2 | **Workers claim only kinds they have a handler for** | After a rollback, jobs the newer release added wait instead of dying as unknown. |
| 2026-10-29 | M1-D2 | **Traits behind `dyn` return boxed futures; no `async-trait` crate** | `async fn` in a trait can't be called through `dyn` on Rust 1.99 (checked); a type alias and `Box::pin` do the same as the crate (vanilla-Rust rule). |
| 2026-10-29 | M1-D2 | **No money columns yet** | Paddle, the merchant of record, keeps amounts. The minor-units rule applies when the first one is added. |
| 2026-10-29 | DEC-07 | **Own sign-in: email link → password → passkey; back office passkey only** | Your D3 notes. The backend rules' "no passwords" line is replaced. |
| 2026-10-29 | M1-D3 | **The password is set after the link, not typed at sign-up** | With a password at sign-up, anyone could register your address with a password they know and wait for you to click "confirm" (pre-hijacking). The sign-up screen can still look like one flow: email, inbox, password, passkey. |
| 2026-10-29 | M1-D3 | **One email-link endpoint for sign-up, sign-in and forgot password, with the same answer** | Nobody can test whether an address has an account; the email's words differ, the response doesn't. |
| 2026-10-29 | M1-D3 | **A guest becomes an account only from its own browser** | The refresh cookie decides, never the link: otherwise someone could ask for a link to your address from their guest and get signed in to your account when you click it. |
| 2026-10-29 | M1-D3 | **Passkeys checked on `ring` with a small own CBOR reader (about 180 lines), no WebAuthn crate** | `webauthn-rs` needs OpenSSL; `ring` is already built for sqlx's TLS and has every signature needed (vanilla-Rust rule). Attestation isn't checked, so no certificate chains. |
| 2026-10-29 | M1-D3 | **New crates: `argon2`, `lettre`, `ring`, `time`** | Argon2id is OWASP's first choice and `ring` has only PBKDF2; SMTP with STARTTLS and login isn't small to write; `ring` was in the tree already; `time` for `timestamptz` (smaller than chrono). |
| 2026-10-29 | M1-D3 | **One session row per browser, keeping the previous refresh token; a 30-s race window** | A reused token is told apart from two tabs refreshing at once without a row per refresh. |
| 2026-10-29 | M1-D3 | **`add_passkey` after every sign-in that didn't use a passkey** | Passkeys are per device and the server can't see what a device has; the web app asks only where the device supports them. |
| 2026-10-29 | M1-D3 | **Back office: passkey only, 5-minute tokens, 12-hour sessions, role in SQL, every write logged before it runs** | From D1's suggestion. Logging first means a write that can't be logged doesn't happen. |
| 2026-10-29 | M1-D3 | **`no-store` responses aren't compressed either** | A racing refresh carries a token but no cookie; `no-store` marks every token- or account-carrying response (BREACH). |
| 2026-10-29 | DEC-14 | **Suggested: guests until the first export, the free test print included** | Keeps the §13 upload target, makes U3 the sign-up step, and Paddle needs an account anyway. Open for your confirmation; D7 enforces it. |

---

## Progress log

| Date       | Task | Note                                                                                               |
| ---------- | ---- | -------------------------------------------------------------------------------------------------- |
| 2026-10-27 | —    | Milestone file created from CLAUDE.md. Found: price conflict, Next.js vs Vite, copied rules. |
| 2026-10-27 | M0-01 | DEC-04 (Next.js) and DEC-05 styling (no Tailwind) decided. CLAUDE.md §7.1 updated.                  |
| 2026-10-27 | M0-01 | DEC-02 prices decided. CLAUDE.md §0, §3, §9.1, §9.2, §9.5, §10, §12 and §15.6 updated.              |
| 2026-10-27 | M0-01 | DEC-03 repo layout decided: monorepo, crates inside the backend. M0-03 steps updated to match.        |
| 2026-10-27 | M0-01 | DEC-05 dependencies decided. CLAUDE.md §7.1, §8.2 and §8.6 updated; HEIC limit noted in §7.1 and E2.   |
| 2026-10-27 | M0-03 | Monorepo, Cargo workspace, Docker image (toolchain/build/runtime), CI workflow, Tailwind removed. First commit still to do. |
| 2026-10-27 | M0-02 | Backend and frontend `.claude/rules` rewritten for Paccradle.                                          |
| 2026-10-27 | M0-04 | Landing, pricing, terms, refund and privacy pages; fake door + waitlist + Plausible events. Checked in the browser (desktop, phone, light/dark). |
| 2026-10-27 | M0-08–10 | Vision spike in `crates/vision`; 5 synthetic tests pass (±0.25 mm). `accuracy` tool for M0-11 ready. |
| 2026-10-28 | M0-04 | Landing redesign: new tokens and swatches, animated dieline hero (scan, draw, fold loop, count-up), rotating headline, crossing tape marquee, bento features, wrapped-package audience cards, floating header, animated dialog. Checked at 1280 px and 375 px, light and dark; tsc, lint and build pass. |
| 2026-10-28 | DEC-13 | Backup AI plan rewritten (§5.4, §8.1, §8.6, §12, §15.5, §16): Hugging Face → Gemini free tier → fal.ai, with a global-visitor rule. Env examples updated (fal endpoint was an image generator). Privacy draft names the 3 providers. Backend rules no longer mention Claude. |
| 2026-10-28 | Fix | Frontend `.env.example`: Paddle vars renamed to `NEXT_PUBLIC_PADDLE_ENVIRONMENT` / `NEXT_PUBLIC_PADDLE_CLIENT_TOKEN` so they reach Paddle.js in the browser. |
| 2026-10-28 | M1-A1 | M1 started. `insert_core` data model with validation and §6.4 starting values; 16 tests pass (JSON round trips, fixture, every validation rule). |
| 2026-10-28 | M1-A2 | Geometry helpers: arc flattening, simplification, offsets and booleans with `i_overlay` (picked over Clipper2 in a spike). Worst offset error 0.002 mm (bar: 0.01 mm). 28 new tests; whole workspace passes in Docker; `insert_core` builds for WASM. |
| 2026-10-28 | M1-A3 | Cutout card generator: holes = outline + clearance (±0.01 mm), finger notches, straightened row layout, fixed-box or grown card. 26 new tests (70 in the crate); whole workspace passes in Docker; WASM builds. |
| 2026-10-28 | M1-A3 | Placement settings: finger notch side + position, and row alignment in a fixed box, all defaulting to the centre. 8 new tests (78 in the crate); workspace passes in Docker; WASM builds. |
| 2026-10-28 | M1-A4 | Partition grid: slotted strips, cells from the product + clearance (stretched in a fixed box), optional frame and finger notches, `Insert.mounts` for 3D. Line-up test passes and fails on deliberately broken slots. 13 new tests (91 in the crate); Docker and WASM pass. |
| 2026-10-28 | M1-A5 | Cradle platform: deck with holes on two folded tube legs, lock tabs through deck slots, 6 hinges. A 3D fold test checks floor, flush tabs, slots and leg clearance, and fails on 4 deliberate breaks. Card and cradle share the fit/grow layout code. 12 new tests (102 in the crate); Docker and WASM pass. |
| 2026-10-29 | M1-A6 | Outer boxes: mailer (FEFCO 0427 style, 15 panels, lock tabs, no glue) and shipping box (FEFCO 0201, 13 panels), sized round the insert; `Design::generate()` returns both. A 3D fold test checks collisions, a clear inside, sides ≥ 90% closed, the outside size and flush tabs, and catches 15 deliberate breaks. The cradle gets the collision check. 13 new tests (115 in the crate); Docker and WASM pass. |
| 2026-10-29 | M1-A6 | Your review: shipping box corners and flap tips didn't line up. Now straight scores, equal walls and flaps; outer flaps turn a board past their crease (`Hinge.offset_mm`). New line-up test and break (116 tests in the crate); Docker and WASM pass. |
| 2026-10-29 | M1-A6 | Your second review: the outer bottom flaps floated below their walls. The bend is now modelled (outer flaps a board deeper), and a new check fails any panel that comes loose from its hinge (boxes and cradle). 17 deliberate breaks caught; 116 tests in the crate; Docker and WASM pass. |
| 2026-10-29 | M1-A7 | Manufacturability checks: 14 rules covering every item in §6.5 plus the A1/A5 notes, each pointing at the piece, the spot and the products. Default designs raise nothing; paperboard's sub-millimetre slots and short C-flute margins are flagged. A card of 125 notched holes checks in 30 ms. 15 new tests (131 in the crate); Docker and WASM pass. |
| 2026-10-29 | M1-A8 | 2D fit check: each product copy green, yellow or red by its gap to the board, grid strips, cradle legs and the insert's edge, against the board's recommended clearance. Red names the clash (board, product, leg, off the insert). Samples all green; 125 copies in 17.6 ms. 7 new tests (138 in the crate); Docker and WASM pass. |
| 2026-10-29 | M1-A9 | Golden tests: 15 SVG drawings (5 products × 3 styles, each with a box), compared within 0.002 mm on the Mac and in Docker. They caught a real bug: shipping boxes at some odd sizes fell apart, because each boolean snapped to its own grid. Booleans now share one grid. 6 new tests (144 in the crate); Docker and WASM pass. Waiting on your look. |
| 2026-10-29 | M1-A9 | Your review: ok. A9 done. |
| 2026-10-29 | M1-A10 | WASM build: new `insert_core_wasm` crate (`generate`, `fits`, `warnings`, JSON in/out), preview geometry as plain straight segments, `npm run build:wasm`, typed loader, and a `/dev/geometry` check (404 in production). 615 KB `.wasm`, 218 KB gzipped. One slider frame takes 2–10 ms in Chrome for 6 products. CI builds the WASM before the frontend checks. 6 new tests; Docker (155), WASM, `tsc`, ESLint and `next build` pass. M1-A done. |
| 2026-10-29 | M1-A10 | Your review: preview curves were jagged when zoomed and copies of one hole differed (0.5 mm grid). Now straight segments within 0.05 mm, points to 0.01 mm: smooth in Chrome at 6× zoom, and copies match. CLAUDE.md §7.4 reworded. 1 new test (156 in Docker); WASM rebuilt (219 KB gzipped). |
| 2026-10-29 | M1-B1 | SVG export: `export::Sheet` lays out each piece once (box pieces mirrored, print side up), `Packaging::to_svg()` writes CUT/CREASE/PERFORATION layers in mm, hairlines, exact geometry and no text. Behind the `export` feature (the API turns it on; the WASM leaves it out). Golden helper now shared. 5 new tests; Docker (161) passes. |
| 2026-10-29 | M1-B2 | PDF export: `Packaging::to_pdf()`, one true-size page with CUT, CREASE (dashed), PERFORATION, DIMENSIONS and LABELS layers, arcs as Bézier curves, written by hand. `pypdf` (strict) reads it; macOS renders it. 5 new tests; Docker (166) passes. |
| 2026-10-29 | M1-B3 | DXF export: `Packaging::to_dxf()`, R12 in mm, CUT/CREASE/PERFORATION layers in red/blue/green, one polyline per path with arcs as bulges, written by hand. `ezdxf` reads and audits it (0 errors) and draws it right. 5 new tests; Docker (171) passes. Waiting on your check in LibreCAD or Inkscape and your cutter app. |
| 2026-10-29 | M1-B4 | Print-at-home test PDF: `Packaging::to_test_print(Paper)`. Every copy at true size; whole pieces share pages, bigger ones are tiled with 10 mm overlaps, trim strips, diamonds and tile names (empty tiles left out); numbered fold circles; instructions page; scale bar; watermark over the lines; preview geometry, no layers. Shared PDF code moved to `export/pdf/`. `pypdf` (strict) reads it; pages rendered and checked. 13 new tests; Docker (184) passes; WASM still has no exporters. |
| 2026-10-29 | M1-B5 | Watermarked PNG preview: `Packaging::to_png(&design)`, drawn with tiny-skia in the site's colours and Geist. Sizes and fit in words, top view coloured by fit (enlarged), every piece flat with numbered folds, legend, watermark over everything. Fold-circle placement now shared with the test print. About 180 ms and 590 KB per picture. 8 new tests (pixel checks, two pixel-tolerance goldens); Docker (192) passes; WASM unchanged. |
| 2026-10-29 | M1-C1 | Production vision crate: `vision::run((input, bytes))` for capture jobs; upload checks from the header (JPEG/PNG/WebP, 40 MP cap, 1000 px minimum) before decoding; EXIF orientation honoured; warnings and errors as codes; results read back from JSON; both tools on the job's path. About 0.5 s per photo. 8 new tests (13 in the crate) pass in Docker. |
| 2026-10-29 | M1-C3 | Manual corrections: a tap (`tap_point_px`, photo pixels) re-runs detection and keeps the outline under it, in both modes; `vision::accept_edit` measures an edited outline as drawn from the stored result, no photo; detected outlines also come in photo pixels (`contour_px`) for the editor. Two new codes (`CAPTURE_NOTHING_AT_TAP`, `CAPTURE_OUTLINE_CROSSES_ITSELF`); `measure --tap`. 13 new tests (26 in the crate, 213 in the workspace) pass in Docker; fmt and clippy clean. |
| 2026-10-29 | M1-C4 | Typed height (`height_mm` input, `vision::set_height`) and a camera estimate from the paper's corners and the EXIF lens (24 mm assumed without it). The tall-product warning now says how far the outline may reach past the edge, and goes away for low products and traces. On exact test photos the camera came out within 0.1 mm of its height and the warning matched a drawn tall cylinder's reach (7.8 and 51.9 mm). `measure --height-mm --focal-35mm`; the accuracy report shows the bound. 8 new tests (34 in the crate, 221 in the workspace) pass in Docker; fmt and clippy clean. DEC-09 built as suggested, waiting on your confirmation. |
| 2026-10-29 | M1-D1 | API skeleton: client surface `/api/client/v1` and back-office surface `/api/admin/v1` (fails closed until D3), each with its own CORS origin and semver contract (`api-version: 1.0.0-beta.1`, `GET …/version`); probes `/healthz`, `/readyz`. Typed config with a Docker-rules `.env` reader; `AppError` codes rendered as localized JSON by one middleware, framework errors included; request IDs (UUID v7), JSON logs, compression without cookies, panic catcher, 30 s timeout, per-IP rate limit, 1 MiB body limit. Deploy, CI smoke test, docs and env examples moved to the new paths; Postman contract dated 2026-10-29. 30 tests pass on the Mac and in Docker; fmt and clippy clean. |
| 2026-10-29 | M1-D2 | Database: 5 add-only migrations for the §8.4 tables plus `jobs` (UUID v7 from the API, `timestamptz`, `updated_at` triggers, checked enum-like text, cascades from users, guests as users without an email, an append-only credit ledger, one credit purchase per Paddle transaction, one row per Paddle event). Pool + migrations at startup, `/readyz` pings Postgres (503 naming the database). Job queue (outbox enqueue in the caller's transaction, SKIP LOCKED claims, 5-minute leases, backoff 10 s → 1 h, dead letters, housekeeping) and in-process worker with typed handlers. `scripts/dev-db.sh`, `.sqlx/` offline cache, new settings in the env examples, Postman file. 52 tests pass against Postgres 18; fmt and clippy clean; a real run migrated a fresh database and survived the database stopping. |
| 2026-10-29 | M1-D3 | Sign-in: email links that confirm the address (sign-up, sign-in, forgot password; same answer either way), Argon2id passwords set after the link, passkeys (ES256/EdDSA/RS256, checked on `ring`), guests that turn into accounts or merge into them from their own browser, JWT access tokens and a rotating `HttpOnly` refresh cookie with reuse detection, an Origin check and sign-in rate limits. Back office: passkey-only sign-in, 12-hour sessions, every write logged before it runs (append-only). SMTP through `lettre`, Mailpit locally (`scripts/dev-mail.sh`). Migration `…_auth.sql`, Postman `auth-2026-10-29`, contracts `1.0.0-beta.2`. 34 new tests (86 in the API crate); fmt, clippy and the Docker workspace run clean; a real run sent the email through Mailpit and signed in. |
