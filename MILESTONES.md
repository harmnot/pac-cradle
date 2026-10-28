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

---

## Current focus

| Item              | Value                                                                       |
| ----------------- | --------------------------------------------------------------------------- |
| Milestone         | **M0: Validate & set up** (your tasks) · **M1: MVP build** started early (see Decision log, 2026-10-28) |
| Next task         | **M1-A2**: geometry helpers (mine) · **M0-07**: test photos (yours), then I run the M0-11 report |
| Waiting on        | You: DEC-01, DEC-06, DEC-08 (confirm the 3 styles), M0-05 settings, M0-06 Paddle, M0-07 photos |
| Blockers          | First commit not made yet (M0-03)                                           |
| Last updated      | 2026-10-28                                                                  |

---

## Where things stand (27 Oct 2026)

| Area                       | State                                                                                                                                                                                                                       |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Plan                       | [CLAUDE.md](CLAUDE.md), status "before validation"                                                                                                                                                                          |
| `pac-cradle/` (this folder) | Monorepo root (DEC-03): `git init` done, **no commits yet**. CI workflow in `.github/workflows/ci.yml` (runs once the repo is on GitHub).                                                                                    |
| `paccradle-backend/`       | Cargo workspace: API placeholder in `src/`, `insert_core` with its data model (M1-A1), and the `vision` spike (M0-08 to M0-10). `insert_core` needs no OpenCV, so plain `cargo test -p insert_core` works on this Mac. Docker toolchain image with OpenCV 4.10. Builds need OpenCV, so on this Mac run cargo through `scripts/in-docker.sh`. |
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
| DEC-07 | Auth (§15.2)                                   | own magic links · hosted auth service                                                         | **Own magic links.** Your backend rules already specify magic link + JWT + rotating refresh cookie. Switch to hosted only if M1 runs late.                              | M1-D3     | open   |
| DEC-08 | First 3 insert styles (§15.3)                  | —                                                                                             | Partition grid, cutout card, cradle platform                                                                                                                            | M1-A      | open   |
| DEC-09 | Height capture in MVP (§15.4)                  | type it · lay-down photo                                                                      | **Type it** (fix B, §5.2). Lay-down photo + pose correction come in M3.                                                                                                  | M1-C4     | open   |
| DEC-10 | Launch timing                                  | public launch in weeks 9–10 (holidays) · move it                                              | **Private beta over the holidays, public launch in the week of 11 Jan 2027.** M3 shifts by about a week.                                                                 | M2        | open   |
| DEC-11 | Languages at launch (§15.7)                    | EN · EN + Bahasa Indonesia                                                                    | **EN at launch.** All strings go through `t()` from day 1 (your rule), so adding ID later is cheap.                                                                     | M2        | open   |
| DEC-12 | Pocket tray / neck holder phase                | §6.2 says v1 · §12 says v2                                                                    | **v2**, following the roadmap (scope risk, §14)                                                                                                                          | M3        | open   |
| DEC-13 | Backup AI model (§15.5)                        | Image generators (SD line art, ControlNet) can't find products · vision-language models       | **Hugging Face `Qwen3-VL-8B-Instruct` → Gemini free tier (not for EEA/UK/Swiss visitors) → fal.ai Moondream 3.** No Claude API. See §5.4. Run all three on the M0 photo set in M3-02. | M3-02     | **decided 2026-10-28** |

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
        - In which currency?
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
- [ ] **A2 · Geometry helpers** · _claude_
    - `kurbo` paths, polygon offset and boolean operations, simplification.
    - Try `i_overlay` and Clipper2 quickly, then pick one.
    - Done when: offset tests on rectangle, circle and concave shapes are within 0.01 mm.
- [ ] **A3 · Cutout card** · _claude_ · §6.2
    - Card size, holes = outline + clearance, finger notches.
- [ ] **A4 · Partition grid** · _claude_ · §6.2
    - Rows × columns, slot width = thickness + tolerance, slot depth = half height + 0.5 mm, optional frame, finger notches.
    - Done when: a test proves the crossing slots line up.
- [ ] **A5 · Cradle platform** · _claude_ · §6.2
    - Deck, fold-down walls, lock tabs, holes taken at deck height. Emits hinges for the 3D fold.
- [ ] **A6 · Outer boxes** · _claude_ · §6.3
    - Mailer (FEFCO 0427) and shipping box (FEFCO 0201), sized around the insert, with hinges.
- [ ] **A7 · Manufacturability checks** · _claude_ · §6.5
    - Every rule in §6.5 produces a warning that points to the piece or feature.
    - Includes open paths and duplicate lines.
- [ ] **A8 · 2D fit check** · _claude_ · §7.3
    - Classify each hole green, yellow or red by clearance.
- [ ] **A9 · Golden tests** · _both_
    - Render every style to SVG snapshots for 5 sample products.
    - You look at them once, then the snapshots guard against regressions.
- [ ] **A10 · WASM build** · _claude_ · §6.1, §7.4
    - `wasm-bindgen` API. A feature flag keeps the exporters out of the WASM build.
    - The Next.js app imports the package built from `paccradle-backend/crates/insert_core` by local path. Nothing is published (DEC-03).
    - Preview coordinates rounded to 0.5 mm.
    - Done when: the frontend can call `generate()` quickly enough for slider drags. Record the bundle size.

### M1-B · Exports, server only (§6.6)

- [ ] **B1 · SVG**: CUT and CREASE groups, mm units · _claude_
- [ ] **B2 · PDF**: vector, CUT and CREASE layers, dimensions (`printpdf` or `krilla`) · _claude_
- [ ] **B3 · DXF**: CUT and CREASE layers in mm · _both_
    - Done when: the file opens correctly in LibreCAD or Inkscape and in one cutter app you use.
- [ ] **B4 · Print-at-home test PDF** · _claude_ · U3
    - Tiled across A4/Letter with overlap and alignment marks, fold steps, watermark.
- [ ] **B5 · Watermarked PNG preview** · _claude_

### M1-C · Vision service (§5)

- [ ] **C1 · Production vision crate** · _claude_
    - Turn the spike into the `vision` crate. It runs as a job and returns §5.5 JSON.
- [ ] **C2 · Trace mode in the production path** · _claude_
    - Includes the pen-width setting.
- [ ] **C3 · Manual corrections** · _claude_ · §3 step 4
    - A tap point re-runs detection. An edited contour is accepted as it is.
- [ ] **C4 · Tall-product warning + height input** · _claude_ · §5.2 fix B, DEC-09
- [ ] **C5 · Accuracy regression in CI** · _claude_
    - The M0-07 photo set runs in CI. The build fails if median error gets worse.

### M1-D · Backend API (§8, following your backend rules)

- [ ] **D1 · API skeleton** · _claude_
    - Typed config, `AppError` with error codes, locales, tracing.
    - tower-http middleware: request ID, CORS, compression, body limit, timeouts, rate limit.
    - Health route.
- [ ] **D2 · Database** · _claude_ · §8.4
    - sqlx migrations for every §8.4 table: UUID v7, `timestamptz`, money in minor units.
    - Job/outbox table and worker: `FOR UPDATE SKIP LOCKED`, retries, dead letter.
- [ ] **D3 · Auth** · _claude_ · DEC-07
    - user required register Passkey if their device support (after success register or login with password), so next time when login , they can use passkey easy, but fallback if fail using passkey 2 times tried, we can send token magic link to email if they forgot their password.
    - when register to our platform, make sure they need confirm emaik when we send magic link
- [ ] **D4 · Uploads & storage** · _claude_ · §8.5
    - Cloudflare R2 provider behind a trait.
    - 10 MB limit, file-type check, pixel cap, rate limits.
    - Strip GPS from EXIF, signed URLs, a job that deletes photos after 30 days.
- [ ] **D5 · Captures endpoints**: create, get, correct; each one starts a vision job · _claude_ · §8.3
- [ ] **D6 · Designs endpoints** · _claude_ · §8.3
    - Create, get and patch, with versions.
    - "My designs" list with `page`, `page_size`, `total_pages`, filtering and sorting (your code-style rule).
- [ ] **D7 · Exports endpoints** · _claude_ · §9.3
    - Entitlement check: Pro, then credit, otherwise `402` with the checkout payload.
    - Export job, then signed download URLs.
- [ ] **D8 · `/me` endpoint** · _claude_
- [ ] **D9 · Tests + contracts** · _claude_
    - `#[sqlx::test]` integration tests for every endpoint.
    - A dated Postman contract file for every endpoint (your rule).

### M1-E · Frontend (§7: Next.js, no Tailwind, dependencies per DEC-05)

- [ ] **E1 · App shell** · _claude_
    - Routes, a typed `fetch` hook for API calls, auth screens, `t()` for strings, design tokens.
    - State uses React built-ins only: `useReducer` and Context (DEC-05).
- [ ] **E2 · Capture flow** · _claude_ · §3 steps 1–3, §7.1
    - Choose the mode: photo, trace or type sizes.
    - Paper defaults by country.
    - Guidance screen (§5.1).
    - Upload: read EXIF with `exifr`, convert HEIC to JPEG on a canvas, resize to ≤ 4000 px, send the EXIF values as separate fields.
    - Only Safari/WebKit can draw HEIC on a canvas. Other desktop browsers get a "please upload a JPEG" message for HEIC files.
- [ ] **E3 · Detection review** · _claude_ · §3 step 4
    - Paper and product outline over the photo, size and confidence, draggable points, tap to pick the product.
- [ ] **E4 · Design choices** · _claude_ · §3 steps 5–8
    - Height, quantity, insert style cards, material, box.
- [ ] **E5 · 2D SVG editor (desktop)** · _claude_ · §7.1
    - Cut lines solid, creases dashed, dimensions shown.
    - Drag products inside the insert, undo/redo.
    - Live preview from the WASM build.
- [ ] **E6 · 3D fold preview** · _claude_ · §7.2
    - Built with `three` + `@react-three/fiber`. Orbit controls (smooth) and the orthographic camera come from `three/addons`, not drei (DEC-05).
    - Hinge tree, panels extruded by board thickness, staged folding, fold slider.
    - Isometric camera, orbit view, exploded view.
    - Partition strips slide into each other's slots.
    - retry mechanism if error to render.
    - show loader while waiting the model render.
- [ ] **E7 · Fit check** · _claude_ · §7.3
    - Colors in the 2D editor. Product prisms in 3D if there's time.
- [ ] **E8 · Warnings panel** (from A7) · _claude_
- [ ] **E9 · Export + checkout** · _claude_ · §9.3
    - Free: watermarked PNG and test PDF (make watermark to not let the AI can not delete it if user want to take the photo and then they use ai to generate what we serve).
    - Paid: Paddle.js overlay checkout, wait for the unlock (SSE or polling `/me`), then download.
- [ ] **E10 · My designs**: list and reopen · _claude_
- [ ] **E11 · Phone layout**: capture and view only · _claude_ · §7.1

### M1-F · Billing (§9)

- [ ] **F1 · Paddle sandbox catalog** · _both_ · §9.2, DEC-02
    - Create the products and prices. Price IDs live in config.
- [ ] **F2 · `BillingProvider` trait + Paddle implementation** · _claude_ · §9.4
- [ ] **F3 · Webhook handler** · _claude_ · §9.4
    - Use the raw body. HMAC-SHA256 over `ts:body`, compared in constant time.
    - Reject old timestamps. Idempotent through `paddle_events`.
    - Return 200 fast and do the work in a job.
- [ ] **F4 · Event handling** · _claude_ · §9.4
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
- [ ] **G2 · Monitoring** · _claude_ · §13
    - Error tracking. Pick a tool. An SDK such as Sentry's is a new runtime dependency, so write down why (DEC-05).
    - Funnel events: visit → capture → design finished → export paid.
- [ ] **G3 · Final legal pages** · _both_ · §14
    - Files are design aids. Always test-print. No claims about shipping protection.
- [ ] **G4 · Account and photo deletion** · _claude_ · §8.5
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
    - Show HN about the Rust/WASM/vision engineering. I'll draft the write-up.

### Gate 2

| Check                                        | Target | Result |
| -------------------------------------------- | ------ | ------ |
| First paying customers (not friends)         | ≥ 1    |        |

---

## M3: v1 (weeks 11–16)

- [ ] **M3-01 · L2 SAM 2** via `ort` (pin the exact version) · _claude_ · §5.4
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
- **Monthly:** re-check the dated facts in CLAUDE.md: Paddle fees, Pacdora features, model prices, crate versions (`opencv`, `ort`).
- **3 months after launch:** apply the revenue rule (§13):
    - ≥ $1,000/month → keep going.
    - < $300/month → change focus.
- **Every price change:** keep every Paddle checkout at $10 or more (§9.1). The $10 single export is already at that line, so no coupons on it.

---

## Parking lot

Ideas that came up mid-build. Review them at each gate.

- **Inch display** for US and other Letter-paper visitors (the site is global). The engine stays in mm; the editor converts (E4/E5).
- **Tidy the backend env examples**: they still mention a PWA and "progress photos" from another project, and the Paddle webhook path `/api/v1/webhooks/paddle` differs from §8.3 (`/api/v1/paddle/webhook`). Pick one path in F3.

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

---

## Progress log

| Date       | Task | Note                                                                                               |
| ---------- | ---- | -------------------------------------------------------------------------------------------------- |
| 2026-10-27 | —    | Milestone file created from CLAUDE.md. Found: price conflict, Next.js vs Vite, copied RepVane rules. |
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
