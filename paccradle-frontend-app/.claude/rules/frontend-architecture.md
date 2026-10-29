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
  (app)/                # capture, editor, exports, my designs, account
  s/[slug]/             # public share page
features/
  <domain>/             # mirrors backend domains: identity, capture, design, editor3d, export, billing, waitlist
    components/
    hooks/
    api.ts              # typed calls to /api/v1
    types.ts            # types written from the Postman contracts
components/ui/          # shared UI: Button, Icon, Section, Dialog
components/motion/      # MotionController (client), SplitText, RotatingWords, CountUp, reveal.ts
lib/
  api-client.ts         # fetch wrapper: auth header, refresh on 401, retries, AbortController
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
- Payments open Paddle Checkout (Paddle.js overlay); never handle card data in the app.
- Production vector files are never generated in the browser; the preview uses `insert_core` WASM, whose drawn lines come back as straight segments within 0.05 mm, points to 0.01 mm (CLAUDE.md §7.4).
- Server data lives in the typed API hooks; React state/Context only for UI, editor and session state.
- Performance budget: marketing pages ship minimal JS; the editor lazy-loads three.js and the WASM; fast first load on mid-range Android phones over 4G.
- Phones get capture and view; desktop gets full editing (CLAUDE.md §7.1).
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
