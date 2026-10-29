# Backend Architecture — Paccradle

> Backend system design for AI assistants. Product plan: `CLAUDE.md` at the repo root (§5 vision, §6 dieline engine, §8 architecture, §9 payments). Task tracker: `MILESTONES.md` at the repo root. Technology choices: `backend-tech-stack.md`. Style: `code-style.md`.

## 1. System overview
- **Backend:** one Rust + Axum service exposing a REST API under `/api/v1`, backed by PostgreSQL, with an in-process background worker for jobs (vision, exports, webhooks, photo purge).
- **Workspace** (`paccradle-backend/` is a Cargo workspace, DEC-03):
  - `src/` — the Axum API (this document).
  - `crates/insert_core/` — geometry core: inserts, boxes, hinges, fit checks, manufacturability warnings; all units mm. Compiled natively for the API and to WASM for the browser preview. Export writers (SVG/PDF/DXF) sit behind a server-only feature flag (CLAUDE.md §6.1).
  - `crates/insert_core_wasm/` — the browser build of `insert_core` (M1-A10): `generate`, `fits` and `warnings` over JSON, drawn lines as straight segments within 0.05 mm, points to 0.01 mm (CLAUDE.md §7.4). It never enables the export writers. Its own lints allow the `unsafe` glue `wasm-bindgen` generates; hand-written `unsafe` is still denied.
  - `crates/vision/` — photo → paper → flattened 10 px/mm image → product outline in mm (CLAUDE.md §5). Pure computation, no I/O beyond the image it is given. The capture job calls `vision::run((input, image_bytes))` (M1-C1) from a blocking task (about half a second per photo); `vision::inspect` checks an upload's format and pixel count without decoding it. Errors carry stable codes (`CAPTURE_*`) that map straight to `AppError` codes.
- **External providers:** Paddle (payments, merchant of record), Cloudflare R2 (photos and export files), a transactional email provider (magic links), and later an AI provider for the L3 vision helper (CLAUDE.md §5.4).
- **Side-effect flow:** request → service writes data + a job row in one transaction → worker picks the job (`FOR UPDATE SKIP LOCKED`) → job calls a service, which calls `vision`, `insert_core` or a third-party provider with retries.

## 2. Core architecture principles
- Follow **Clean Architecture** and **Separation of Concerns**.
- Every module has a **single responsibility**.
- Dependencies always flow inward:
  - `handlers -> service -> repository`
  - `service -> third_party`
  - `service -> engines` (`insert_core`, `vision`)
  - `repository -> database`
  - `jobs -> service` (the worker uses services exactly like handlers do)
- **Handlers must never access repositories, engines or third-party modules directly.**
- **Repositories must never call services, engines or third-party providers.**
- **Third-party modules and engines are called only from services.**
- Business logic belongs **only inside services**. Geometry and image math belong **only inside the engine crates**.
- Database logic belongs **only inside repositories**.
- HTTP-specific logic belongs **only inside handlers**.
- Every service, repository and third-party provider is defined by a **trait**, so implementations can be swapped and mocked in tests. Engines are plain functions over plain data and need no trait.

## 3. Dependency flow

```text
HTTP Request                    Job queue (Postgres)
      │                                │
      ▼                                ▼
  Handlers                           Jobs
      │                                │
      └───────────────┬────────────────┘
                      ▼
                   Service
                      │
                      ├────────► Engines (insert_core, vision)
                      ├────────► Third Party (Paddle, R2, Email, AI)
                      ▼
                 Repository
                      │
                      ▼
                 PostgreSQL
```

**Allowed:** Handlers → Services · Jobs → Services · Services → Repositories · Services → Engines · Services → Third Party · Repositories → Database

**Forbidden:** Handlers → Repository/Engine/Third Party · Jobs → Repository/Engine/Third Party · Repository → Service · Repository → Handler · Third Party → Repository · Third Party → Handler · Service → Handler

## 4. Folder structure

```text
src/
├── main.rs                  # startup: config, DB pool, state, router, worker
├── app_state.rs             # Arc<AppState>: services wired with repositories and providers
├── config/                  # typed AppConfig loaded from environment
├── error/
├── locales/
├── models/
├── repository/
├── service/
├── third_party/
├── schemas/
├── handlers/
├── jobs/
├── middleware/
└── shared/
crates/insert_core/          # geometry engine (native + WASM)
crates/vision/               # vision engine (OpenCV)
migrations/                  # sqlx migrations
postman/                     # dated API contract files (see code-style.md)
tests/                       # integration tests (real Postgres)
```

### 4.1 `src/error`
Centralized error management.

```text
error/
├── mod.rs
├── app_error.rs             # AppError enum + IntoResponse
└── error_code.rs            # unique error codes, e.g. CAPTURE_NOT_FOUND, EXPORT_PAYMENT_REQUIRED
```

Use for: validation errors, repository/database errors, service/use-case errors, authentication errors, authorization errors (e.g. design owned by another user), entitlement errors (402 with checkout payload), vision/geometry errors, payment/webhook errors, third-party errors, unexpected/internal errors.

Every error contains: unique error code, HTTP status mapping, localized message key, optional metadata, original source error (logged, never returned).

Response body: `{ "error": { "code": "...", "message": "<localized>", "meta": {...} } }`. Never expose raw framework, database, OpenCV or provider errors to clients.

### 4.2 `src/locales`
Localized messages for errors and emails.

```text
locales/
├── mod.rs
├── en.rs
└── shared.rs                # resolve language from Accept-Language / user preference; placeholder formatting
```

English at launch (DEC-11). Placeholder variables (e.g. `{design_name}`). New languages (Bahasa Indonesia first) are added without touching services.

### 4.3 `src/models`
Domain entities representing database tables (CLAUDE.md §8.4).

```text
models/
├── mod.rs
├── user.rs                  # User
├── session.rs               # MagicLinkToken, RefreshToken
├── capture.rs               # Capture (photo, EXIF, mode, paper, vision result, purge_after)
├── product.rs               # Product (contour_mm, bbox_mm, shape, height_mm)
├── design.rs                # Design (spec jsonb = insert_core::Design, version, share_slug)
├── export.rs                # Export (formats, status, files)
├── entitlement.rs           # DesignUnlock, CreditLedgerEntry, Subscription
├── paddle_event.rs
└── job.rs
```

Rules: no HTTP logic, no repository logic, no service logic. Money fields use integer minor units (`i64`, USD cents) plus currency. Lengths are `f64` millimetres.

### 4.4 `src/repository`
Database communication layer (SQLx).

```text
repository/
├── mod.rs
├── user/
│   ├── mod.rs
│   ├── user_repository.rs
│   └── user_repository_trait.rs
├── session/
├── capture/
├── product/
├── design/
├── export/
├── entitlement/             # design_unlocks, credit_ledger, subscriptions
├── paddle_event/
└── job/
```

Each folder follows the same pattern: `mod.rs`, `<name>_repository.rs`, `<name>_repository_trait.rs`.

Responsibilities: CRUD, joins, pagination, credit balance (`SUM(delta)`), job claiming (`FOR UPDATE SKIP LOCKED`), transactions (exposed so services can run several writes atomically).

Rules: only database mapping; no business logic, no validation, no authorization, no third-party calls; called only from services.

### 4.5 `src/service`
Business logic layer — one folder per Paccradle domain.

```text
service/
├── mod.rs
├── identity/                # magic links, sessions, /me
│   ├── mod.rs
│   ├── identity_service.rs
│   └── identity_service_trait.rs
├── capture/                 # upload checks, vision jobs, corrections (calls vision)
├── design/                  # create/update designs, versions, share links (calls insert_core)
├── export/                  # entitlement check, export jobs, signed URLs (calls insert_core exporters)
├── billing/                 # Paddle checkout payloads, webhooks, credits, unlocks, subscriptions
└── privacy/                 # photo purge, account and data deletion
```

Each folder follows the same pattern: `mod.rs`, `<domain>_service.rs`, `<domain>_service_trait.rs`.

Responsibilities: business rules, use cases, validation flow, authorization (a user only sees their own captures, designs and exports; share links are read-only), repository orchestration, engine calls, third-party orchestration, transactions, enqueueing jobs for side effects.

Rules: can call repositories, engines and third-party providers; can call another domain's service trait (never its repository); must not know HTTP details; called only from handlers and jobs.

### 4.6 `src/third_party`
External provider integrations, each behind a trait.

```text
third_party/
├── mod.rs
├── payment/
│   ├── mod.rs
│   ├── billing_provider_trait.rs   # BillingProvider (CLAUDE.md §9.4) — swap to another merchant of record later
│   └── paddle.rs                   # checkout data, subscriptions API, webhook signature verification
├── storage/
│   ├── mod.rs
│   ├── storage_trait.rs
│   └── cloudflare_r2.rs            # uploads, short-lived signed URLs, deletes
├── email/
│   ├── mod.rs
│   ├── email_sender_trait.rs
│   └── <provider>.rs               # magic links
└── ai/                             # v1 (M3-02): L3 helper — product pick, photo problems, SAM prompts
    ├── mod.rs
    ├── vision_helper_trait.rs
    ├── vision_helper_chain.rs      # tries the providers in order; skips Gemini free tier for EEA/UK/Swiss visitors
    ├── hugging_face.rs             # Inference Providers router (OpenAI chat format)
    ├── gemini.rs                   # Gemini API free tier, paced to the daily quota
    └── fal.rs                      # fal.ai Moondream 3 detect/query
```

Rules: only services may call this module; never called by handlers, jobs or repositories. Providers return typed results and map their errors into `AppError` third-party errors.

### 4.7 `src/schemas`
Request/response contracts. The API contract is documented in dated Postman files under `postman/` (see `code-style.md`); the frontend writes its TypeScript types from those files.

```text
schemas/
├── mod.rs
├── common.rs                # pagination (page, page_size, total_pages), error body, money
├── identity.rs              # MagicLinkRequest, SessionResponse, MeResponse
├── capture.rs               # CaptureResponse (CLAUDE.md §5.5 JSON), CorrectionRequest
├── design.rs                # CreateDesignRequest, DesignResponse, DesignListQuery
├── export.rs                # CreateExportRequest, ExportResponse, CheckoutRequired (402)
└── webhooks.rs              # Paddle webhook payloads
```

Responsibilities: request DTOs, response DTOs, HTTP request validation (validator), mappers from models to responses.

### 4.8 `src/handlers`
HTTP layer — one folder per domain.

```text
handlers/
├── mod.rs                   # builds the /api/v1 router from every *_route.rs
├── identity/
│   ├── mod.rs
│   ├── identity_handler.rs
│   └── identity_route.rs
├── capture/                 # multipart upload (15 MB limit)
├── design/
├── export/
├── billing/                 # customer portal link
├── share/                   # public read-only share data
└── webhooks/                # Paddle: verify raw body, store event, return 200
```

Each folder follows the same pattern: `mod.rs`, `<domain>_handler.rs`, `<domain>_route.rs`.

Responsibilities: parse requests, headers and auth extractors; determine preferred language; validate the request schema; call services; return JSON responses and HTTP status codes.

Rules: minimal logic; never call repositories, engines or third-party modules directly.

### 4.9 `src/jobs`
Background worker that processes the Postgres job table.

```text
jobs/
├── mod.rs
├── worker.rs                # claims jobs, retries with backoff, dead-letters failures
├── run_vision.rs            # calls capture service
├── run_export.rs            # calls export service
├── process_webhooks.rs      # calls billing service for stored Paddle events
├── reconcile_billing.rs     # daily: calls billing service to repair subscription drift
└── purge_photos.rs          # daily: calls privacy service (photos older than 30 days)
```

Rules: jobs behave like handlers for background work — they only call services; jobs are idempotent and safe to retry.

### 4.10 `src/middleware`
Cross-cutting HTTP concerns built on tower/tower-http: request ID, tracing, CORS (only the app's own origin), compression, body limits, rate limiting (uploads per user and per IP), timeouts, authentication (JWT extraction into `AuthUser`), language detection.

### 4.11 `src/shared`
Reusable utilities with no business logic.

Examples: string helpers, date/time helpers, money helpers (minor units, formatting), unit helpers (mm), constants, crypto (token hashing, HMAC-SHA256 with constant-time compare), logger setup, ID generation (UUID v7), pagination helpers, common types.

## 5. Data model (core tables)
See CLAUDE.md §8.4 for the full list:
`users`, `captures`, `products`, `designs`, `design_unlocks`, `credit_ledger`, `subscriptions`, `paddle_events`, `exports`, plus `jobs`, `magic_link_tokens` and `refresh_tokens`.

## 6. Integration rules
- **Paddle webhooks:** the handler takes the **raw body**, the service verifies `Paddle-Signature` (HMAC-SHA256 over `ts:body`, constant-time compare, reject old timestamps), stores the event in `paddle_events` keyed by `event_id`, returns 200 quickly, and a job processes it. Processing must be idempotent (CLAUDE.md §9.4).
- **Entitlements:** export requests check Pro → credits → otherwise return 402 with the checkout payload (CLAUDE.md §9.3). Production vector files are only generated on the server after this check (CLAUDE.md §7.4).
- **Outgoing side effects** (emails, vision runs, exports, AI calls) go through the job table, never inline in request handlers.

## 7. Security & privacy rules
- **Uploads:** 15 MB maximum, file-type check, pixel-count cap (decompression bombs), rate limits (CLAUDE.md §8.5).
- **Photos:** strip GPS from EXIF, store as private R2 objects, delete after 30 days; keep only outlines and sizes. Photos are only sent to an AI provider when the user hasn't opted out (CLAUDE.md §5.4).
- **Paid files:** short-lived signed URLs only.
- Refresh tokens issued as `HttpOnly`, `Secure`, `SameSite` cookies scoped to the API domain; CORS allows only the app's own origin.
- Support data export and account deletion for users.
- Secrets only from environment/secret store; never committed.
- No user-supplied code ever runs on the server.

## Related files
- `CLAUDE.md` (repo root) — product plan
- `MILESTONES.md` (repo root) — tasks, decisions, progress
- `backend-tech-stack.md` — backend languages, libraries and coding rules
- `backend-architecture.md` — backend layers, folder structure, data model, integrations and security (this file)
- `code-style.md` — coding style
