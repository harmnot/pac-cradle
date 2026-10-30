# Frontend Architecture — Paccradle

> Frontend system design for AI assistants. Product plan: `CLAUDE.md` at the repo root (§3 user flow, §7 editor and 3D, §11 SEO pages). Technology choices: `frontend-tech-stack.md`.

## 1. Overview
One Next.js codebase serves three things:
- **Marketing:** landing page, pricing, legal pages, how-to guides and SEO template pages (CLAUDE.md §11). Static HTML, almost no client JavaScript.
- **App:** capture → detection review → design choices → 2D editor + 3D fold preview → export (CLAUDE.md §3). Client-heavy; the WASM geometry core and three.js load only here.
- **Share pages:** view-only 3D preview for a shared design (`/s/{slug}`).

## 2. Folder layout
```
app/
  layout.tsx            # root: fonts, tokens, base styles, analytics script
  (marketing)/          # landing, pricing, legal, guides, SEO template pages
    layout.tsx          # site header + footer (legal links live in the footer menu)
  (app)/                # SessionProvider + noindex for every app page (M1-E1)
    (auth)/             # AuthShell (form + kraft art panel): sign-in, sign-up, auth/link, security/{password,passkey}
    (workspace)/        # AppHeader: app (workspace), account, capture, design/new (choices), design/[designId]; editor, exports, my designs join here
  s/[slug]/             # public share page
features/
  <domain>/             # mirrors backend domains: identity, capture, design, editor3d, export, billing, waitlist
    components/
    hooks/
    api.ts              # typed calls to /api/v1
    types.ts            # types written from the Postman contracts
components/ui/          # shared UI: Button, IconButton, Icon, Notice, TextField, Skeleton, Section, ChoiceCards, ChoiceChips (radio groups, radio-keys.ts), NumberStepper
components/motion/      # MotionController (client), SplitText, RotatingWords, CountUp, reveal.ts
lib/
  api-client.ts         # fetch wrapper: token in memory, one shared refresh on AUTH_REQUIRED (navigator.locks across tabs), read retries, abort → ApiResult
  use-api.ts            # useApiQuery: in-memory cache, abort, reload
  errors.ts             # describeError: error.<CODE> from locales, else the API's message
  base64url.ts          # for the WebAuthn JSON fallback
  analytics.ts          # Plausible custom events
  i18n.ts               # t() helper + Intl formatters
  config.ts             # typed public env config (NEXT_PUBLIC_*)
styles/
  tokens.css            # CSS custom properties (colors, swatches, spacing, type scale, motion, dark mode)
  base.css
  motion.css            # scroll-reveal states driven by data-reveal / data-revealed
locales/
  en.json
content/
  en/                   # long-form content: legal pages, guides
public/
```

## 3. App rules
- Auth: access token in memory; refresh token in an `HttpOnly`, `Secure`, `SameSite` cookie set by the API; never store tokens in `localStorage`.
- Session (M1-E1): `features/identity/session.tsx` (`useSession()`) owns who is signed in; the token stays in `lib/api-client`. The session lives in the browser only (the cookie belongs to the API's host), so app pages prerender as static shells and never read `cookies()` or `headers()`. Account-only pages wrap their content in `RequireAccount`; actions that need any session (capture, design) call `ensureSession()`, which starts a guest. API calls go through `apiJson`/`apiNoContent` (or `useApiQuery` for reads) and show failures with `describeError`. Never call `fetch` on the API directly.
- Sign-in routes and steps live in `features/identity/routes.ts` (`AppRoute`, `routeAfterSignIn`, `safeNextPath` for `?next=`).
- Navigation motion: wrap each app `page.tsx` in `PageTransition`, tag links and `router.push/replace` with `transitionTypes` (`TransitionType.Forward`/`Back`), and swap in-page steps with `StepTransition` inside `startTransition` (`styles/transitions.css`). Next keeps recently visited pages mounted but hidden: don't rely on a page's DOM being gone after navigation.
- Pages that read `useSearchParams()` sit inside `<Suspense>` (Cache Components); values that change per visit (`Date.now()`) are read in effects, handlers or loaders, never during render. Anything reading the address (`usePathname`, `useParams`) on a dynamic route (`/capture/[captureId]`) must be inside `<Suspense>` too: the workspace layout wraps `AppHeader` for that reason.
- Captures (M1-E2): photos are prepared in the browser by `features/capture/prepare-photo.ts` (exifr `lite`, per-block picks with GPS off, upright via `createImageBitmap`, ≤ 4000 px JPEG) and uploaded with `uploadCapture`; read and poll a capture with `useCapture(id)`; draw an outline over the photo with `OutlineFigure` (coordinates in photo pixels). Never send a photo straight from the file input, and never send more than make, model and the 35 mm focal length from its EXIF.
- Detection review (M1-E3): `OutlineReview` puts the tap and point tools under the photo; tools draw through `OutlineFigure`'s `overlay` (photo pixels, with `unitsPerPx` for sizes that stay the same on screen and `toPhoto` for pointers). Corrections go through `correctCapture` and back into the page with `useCapture().apply`. Plane maths (photo ↔ sheet homography, crossings, sizes as vision measures them) lives in `features/capture/geometry.ts`; the point editor is a pure reducer with its history (`outline-editor.ts`). Check an outline with vision's own rules before sending it, and keep dragged points on the sheet.
- Design choices (M1-E4): `/design/new` (`?capture={id}`, else the typed product) builds the engine's `Design` with `buildDesign` from the page's reducer (`design-choices-state.ts`: unpicked choices follow `recommendStyle`/`recommendBoard`/`recommendBoxBoard`); defaults live in `features/design/choices.ts` and are tested against the real WASM (`choices.test.ts`), so change them there. Load the engine with `useInsertCore()`, run it with `runPreview`, draw it with `PackagingPreview` (`preview.ts`: `planView`, `layOutPieces`). Make picks inside `startTransition` so the preview morphs; typing stays synchronous. A capture's typed height goes through `correctCapture({ kind: "height" })` once typing settles. Save with `createDesign`, then open `designPath(id)`.
- Browser-only values (storage, `navigator`) never go in a reducer's initial state behind `typeof window`: the server and the first client render must agree. Read them with `useSyncExternalStore` and a server snapshot (`useDefaultPaper`, `useKeptTypedProduct`).
- Lines drawn on paper (`--swatch-*` surfaces: board, kraft) use `--color-ink`, which stays dark in both modes; `--color-cut` follows the theme and is for lines on the page itself.
- Payments open Paddle Checkout (Paddle.js overlay); never handle card data in the app.
- Production vector files are never generated in the browser; the preview uses `insert_core` WASM, whose drawn lines come back as straight segments within 0.05 mm, points to 0.01 mm (CLAUDE.md §7.4).
- Server data lives in the typed API hooks; React state/Context only for UI, editor and session state.
- Performance budget: marketing pages ship minimal JS; the editor lazy-loads three.js and the WASM; fast first load on mid-range Android phones over 4G.
- Phones get capture and view; desktop gets full editing (CLAUDE.md §7.1). The outline check right after the photo (tap, adjust the points) works on phones too (M1-E3 decision); the 2D insert editor is desktop-only.
- Accessibility: semantic HTML, keyboard navigation, visible focus, sufficient contrast, readable font sizes on mobile.
- Branding (from the brand reference images, 2026-10-27): sustainable packaging — oat and cream paper, kraft board, terracotta and rust ties, sage/eucalyptus green, espresso ink. Defined once in `styles/tokens.css`:
  - `--color-*` tokens follow light/dark mode; `--swatch-*` tokens are physical paper colours that stay the same in both modes (use them for package-coloured surfaces, with `--color-on-forest` or `--color-ink` text).
  - Type: Fraunces (display, headings) + Geist (UI, body) + Geist Mono (measurements), loaded with `next/font`.
  - Recurring motifs: paper grain texture, torn-paper section edges, cream label bands with uppercase tracked text, fabric ties, eucalyptus line art (`Botanical`), dieline drawings (cut solid, crease dashed terracotta, fit rings green/yellow/red).
- Motion ("ultra hi-fi", no animation library — DEC-05):
  - Hero and one-off entrances: CSS keyframes. Scroll reveals: add `data-reveal` (rise), `data-reveal="words"` (SplitText) or `data-reveal="draw"` (line drawing); `MotionController` sets `data-revealed`. Stagger with `revealDelay(index)`.
  - Hover: lift with the individual `translate`/`rotate`/`scale` properties (never `transform`, which the reveal animates); spotlight with `data-glow`; hero parallax with `data-parallax`.
  - Always respect `prefers-reduced-motion`; content must stay visible if JavaScript fails (the boot script in `app/layout.tsx` handles that).
- Server and client modules: constants shared by server and client components (e.g. `WaitlistTone`) live in plain modules such as `types.ts`, never in a `"use client"` file — a server component only gets a client reference from those.
- Paddle domain approval (CLAUDE.md §9.1): pricing page, Terms with the legal name, Refund Policy and Privacy Policy must stay reachable from the site menu.
