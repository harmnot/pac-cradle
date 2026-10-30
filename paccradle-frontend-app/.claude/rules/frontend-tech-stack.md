# Frontend Tech Stack — Paccradle

> Frontend technology choices and rules for AI assistants. Product plan: `CLAUDE.md` at the repo root (§7). Task tracker: `MILESTONES.md`. Frontend design: `frontend-architecture.md`. Style: `code-style.md`.

## 1. Dependency rule: vanilla first (DEC-05)
- **No Tailwind, no CSS frameworks, no component libraries, no state/data-fetching/form/chart libraries.**
- If something can be built with React built-ins, Next.js built-ins or standard web APIs, build it in vanilla code.
- **Allowed runtime dependencies:**
  - `next`, `react`, `react-dom`
  - `three`, `@react-three/fiber` (3D fold preview; no drei — controls and cameras come from `three/addons`)
  - `exifr` (read EXIF from JPEG/HEIC before resizing)
  - Paddle.js (loaded from Paddle's CDN, required by Paddle)
  - the `insert_core` WASM package (built from `paccradle-backend/crates/insert_core_wasm` by `npm run build:wasm` into `wasm/insert-core/`, generated and not committed; load it through `features/design/insert-core.ts`)
  - the Plausible analytics script tag
- Any other runtime dependency needs an explicit, written justification in `MILESTONES.md` (Decision log).
- Dev-only tooling (types, lint, tests) is allowed because it ships nothing to users.

## 2. Stack
| Concern            | Choice |
|--------------------|--------|
| Framework          | Next.js (App Router) + React + TypeScript (strict mode) (DEC-04) |
| Rendering          | Server Components by default; static pages for marketing, legal and SEO template pages; `"use client"` only where interaction needs it |
| Routing            | Next.js file-system routes; route groups `(marketing)` and `(app)` |
| Server data        | Native `fetch` in a typed API module + small custom hooks with in-memory caching, retries and abort via `AbortController` |
| Client state       | React built-ins: `useState`, `useReducer` (editor, with an undo/redo history), Context (session) |
| Forms & validation | Native HTML form validation (`required`, `pattern`, `min`, `max`) + small TypeScript validation functions |
| Styling            | Vanilla CSS: design tokens in `styles/tokens.css`, CSS Modules (`*.module.css`) per component, mobile-first media queries, light/dark via `prefers-color-scheme` |
| Type               | Fraunces (display) + Geist + Geist Mono via `next/font/google` (self-hosted at build time) |
| Motion             | CSS keyframes and transitions; one client `MotionController` (IntersectionObserver reveals, scroll state, pointer spotlight/parallax). No animation library. Honour `prefers-reduced-motion` |
| 2D editor          | Hand-built SVG React components (cut solid, crease dashed, dimensions) |
| 3D preview         | `three` + `@react-three/fiber`, lazy-loaded with `next/dynamic` (`ssr: false`) |
| Geometry preview   | `insert_core` WASM, lazy-loaded on editor routes only |
| Photo upload       | `exifr` + canvas resize to ≤ 4000 px; HEIC converts on a canvas only in Safari/WebKit — other browsers get a "please upload a JPEG" message |
| i18n               | JSON dictionaries per language in `locales/` + a tiny `t()` helper; dates, numbers and currency via the native `Intl` API. English at launch (DEC-11) |
| Analytics          | Plausible script tag + custom events through `lib/analytics.ts` |
| API types          | Hand-written in `features/<domain>/types.ts` from the dated Postman contracts in `paccradle-backend/postman/` |
| Payments UI        | Paddle.js checkout overlay |
| Testing            | Vitest (installed in M1-E1: `pnpm test`, Node environment, logic in `lib/` and `features/*/`; `*.test.ts` next to the code) + React Testing Library later; Playwright for end-to-end (dev-only, not installed yet) |
| Quality            | ESLint + Prettier, `tsc --noEmit` in CI |
| Node               | Version from `.nvmrc` (`nvm use`) |

## 3. Coding rules
- Function components and hooks only; no class components.
- No `any` in TypeScript.
- No inline styles except for dynamic values (e.g. SVG sizes); all colors, spacing and type come from CSS custom properties.
- All user-facing UI text goes through `t()`; no hard-coded strings in components. Long-form content (legal pages, guides) lives in `content/` per language.
- Never store tokens or photos in `localStorage`.

## Related files
- `CLAUDE.md` (repo root) — product plan
- `MILESTONES.md` (repo root) — tasks, decisions, progress
- `frontend-tech-stack.md` — frontend libraries, dependency rule and tooling (this file)
- `frontend-architecture.md` — app structure and frontend rules
- `code-style.md` — coding style
