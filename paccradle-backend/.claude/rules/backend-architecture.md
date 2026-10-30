# Backend Architecture — Paccradle

> Backend system design for AI assistants. Product plan: `CLAUDE.md` at the repo root (§5 vision, §6 dieline engine, §8 architecture, §9 payments). Task tracker: `MILESTONES.md` at the repo root. Technology choices: `backend-tech-stack.md`. Style: `code-style.md`.

## 1. System overview
- **Backend:** one Rust + Axum service exposing two REST surfaces, `/api/client/v1` (the web app's users) and `/api/admin/v1` (the back office), plus the probes `/healthz` and `/readyz` (§4.8), backed by PostgreSQL, with an in-process background worker for jobs (vision, exports, webhooks, photo purge).
- **Workspace** (`paccradle-backend/` is a Cargo workspace, DEC-03):
  - `src/` — the Axum API (this document).
  - `crates/insert_core/` — geometry core: inserts, boxes, hinges, fit checks, manufacturability warnings; all units mm. Compiled natively for the API and to WASM for the browser preview. Export writers (SVG/PDF/DXF) sit behind a server-only feature flag (CLAUDE.md §6.1).
  - `crates/insert_core_wasm/` — the browser build of `insert_core` (M1-A10): `generate`, `fits` and `warnings` over JSON, drawn lines as straight segments within 0.05 mm, points to 0.01 mm (CLAUDE.md §7.4). It never enables the export writers. Its own lints allow the `unsafe` glue `wasm-bindgen` generates; hand-written `unsafe` is still denied.
  - `crates/photo_file/` — uploaded photos as files (M1-D4), pure Rust: `inspect` reads the format and pixel size from the header (JPEG, PNG, WebP; 40 MP cap, 1000 px minimum) and `clean` copies the file keeping only the picture, its colour profile and its EXIF orientation. The API calls both at upload time without linking OpenCV; `vision` uses the same header code.
  - `crates/vision/` — photo → paper → flattened 10 px/mm image → product outline in mm (CLAUDE.md §5). Pure computation, no I/O beyond the image it is given. The capture job calls `vision::run((input, image_bytes))` (M1-C1) from a blocking task (about half a second per photo); `vision::inspect` (from `photo_file`) checks an upload's format and pixel count without decoding it. Corrections (M1-C3): a tap re-runs `vision::run` with `CaptureInput.tap_point_px`; `vision::accept_edit((stored_result, points_px))` measures an edited outline from the stored result, with no photo. Taps and outlines are in photo pixels. Height (M1-C4): `CaptureInput.height_mm`, or `vision::set_height((stored_result, height_mm))` with no photo, rechecks the tall-product warning against the camera the result carries. Errors carry stable codes (`CAPTURE_*`) that map straight to `AppError` codes.
- **External providers:** Paddle (payments, merchant of record), Cloudflare R2 (photos and export files), a transactional email provider over SMTP (sign-in links, M1-D3), and later an AI provider for the L3 vision helper (CLAUDE.md §5.4).
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
├── main.rs                  # startup: config, logging, listener, graceful shutdown (later: DB pool, worker)
├── lib.rs                   # app(state): handlers::router + middleware::stack (tests drive it with oneshot)
├── app_state.rs             # AppState: config now; services wired with repositories and providers from M1-D2
├── config/                  # typed AppConfig from the environment, then ./.env (env_file.rs, Docker --env-file rules)
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
crates/photo_file/           # upload header checks and metadata cleaning (pure Rust, M1-D4)
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

How it flows (M1-D1): handlers and middleware return `AppError` (`AppError::new(code).with_meta(k, v).with_source(e)`). Its `into_response` logs the source (5xx at error, 4xx at debug) and leaves an `ErrorReport` (code + meta) in the response's extensions. The `error_body` middleware writes the body from it in the caller's language. Any other 4xx/5xx (an extractor's rejection, a 405, a caught panic) gets the code closest to its status (`ErrorCode::from_status`), so framework text never reaches the caller. Codes are stable once released: a new meaning gets a new code (a minor version bump). Vision errors (`CAPTURE_*`) join `ErrorCode` when the API links the vision crate (M1-D5).

### 4.2 `src/locales`
Localized messages for errors and emails.

```text
locales/
├── mod.rs
├── en.rs
└── shared.rs                # resolve language from Accept-Language / user preference; placeholder formatting
```

English at launch (DEC-11). Placeholder variables (e.g. `{design_name}`) take the error's meta values (`locales::fill`). `Locale::from_headers` picks the first offered language in `Accept-Language`, else English; `error_body` also leaves the `Locale` in the request's extensions for handlers (emails). New languages (Bahasa Indonesia first) are added as a file plus a `Locale` variant, without touching services.

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

Each model arrives with the task that first reads or writes its table. Built (M1-D2): `job.rs`: `JobKind` (a job's name, payload type and attempts), `NewJob::of::<K>(&payload)`, `Job` (a claimed job), `JobError` (`Retry` or `GiveUp`), `JobOutcome`, `Tidied`. Enum-like columns hold lower-case snake_case text in the database (`'a4'`, `'l1'`); the model maps them to Rust enums by implementing `TextEnum` (`ALL` + `as_str`; `from_text` comes free and turns an unknown stored value into a 500).

Built (M1-D3): `user.rs` (`User` with `has_password` and `passkey_count`, `Role`, `AccountRole` guest/user/admin, `NewUser`), `session.rs` (`Session`, `NewSession`, `RefreshMatch`, `Rotation`, `SignInMethod`, `AuthUser`, `SignedIn`, `NextStep`), `email_link.rs` (`EmailLink`, `EmailLinkPurpose`, the `SendEmailLink` job kind), `passkey.rs` (`Passkey`, `NewPasskey`, challenges, `PasskeyOptions`, the decoded browser answers), `audit.rs` (`AdminWrite`).

Built (M1-D4): `paper.rs` (`Paper`: `users.paper_default` and `captures.paper`), `capture.rs` (`CaptureStatus`, `CaptureMode`, `CaptureSettings` in `vision::CaptureInput`'s JSON shape for `captures.input`, `CameraExif` for `captures.exif` (make, model, focal length; never a location), `NewCapture`, `Capture`, `CreatedCapture`, `DuePhoto`, the `PurgePhotos` job kind, `PHOTO_RETENTION` 30 days and the setting ranges vision checks again). `job.rs` gained `Schedule` (a kind that runs by itself every so often).

Built (M1-D5): `capture.rs` gained the full row (`run`, `result` as the §5.5 JSON, `confidence`, `level_used` as `VisionLevel`, `error` as `StoredError` code + meta, `storage_key`), `tap_point_px` in `CaptureSettings`, `CaptureView` (a capture and its signed photo link), `Correction` (tap, edit, height) and `Corrected` (done at once, or a run is pending), and the `RunVision` job kind (`{capture_id, run}`). The setting ranges are vision's own now (`vision::CaptureInput::validate`, called by the capture service).

Built (M1-D6): `page.rs` (`PageRequest` with `offset`/`limit`, `Page<T>` with `total_pages`, `SortOrder`; `MAX_PAGE_SIZE` 100, `DEFAULT_PAGE_SIZE` 20: every list uses them), `design.rs` (`SavedDesign` holding the typed `insert_core::Design`, `NewDesign`, `DesignChange` with the version it was made from, `DesignSummary`, `InsertStyle`, `DesignSort`, `DesignListQuery`).

Built (M1-D7): `entitlement.rs` (`DesignUnlock`, `NewUnlock`, `UnlockSource`, `CreditSpend`, `PaidWith`, `EDIT_WINDOW` 30 days), `export.rs` (`ExportFormat` with `is_paid`, `ExportStatus`, `NewExport` with the spec copy, `Export`, `ExportFile`, `ExportLink`, `ExportView`, `ExportSummary`, `ExportListQuery`, the `RunExport` job kind).

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

How they are written (M1-D2):
- `database.rs`: `connect` (pool: `DATABASE_MAX_CONNECTIONS`, 5 s acquire timeout, 30 s `statement_timeout`, `application_name = paccradle-api`), `connect_lazy` (tests), `migrate` (the `migrations/` files, built into the binary, applied at startup when `DATABASE_RUN_MIGRATIONS`).
- A trait method returns `shared::future::BoxFuture` (`Box::pin(async move { … })`): `async fn` in a trait can't be called through `dyn` (Rust 1.99), and `AppState` holds `Arc<dyn …>`. The Postgres implementation is `Pg<Name>Repository`.
- A write that must commit with the caller's other writes takes `conn: &mut PgConnection`; the service passes its open transaction (`&mut tx`). Other methods use the pool.
- Queries use `sqlx::query!` / `query_as!`, checked at compile time against the database, or against the committed `.sqlx/` cache when `DATABASE_URL` is unset or `SQLX_OFFLINE=true` (CI, Docker). After changing a query, run `DATABASE_URL=$(scripts/dev-db.sh) cargo sqlx prepare -- --all-targets` and commit `.sqlx/`.
- No SQL built from strings: sqlx 0.9 only takes `&'static str` SQL unless wrapped in `AssertSqlSafe`, which production code doesn't use.
- `sqlx::Error` converts into `AppError` (`?`): a database that can't be reached is 503 `SERVICE_UNAVAILABLE` (`meta.dependency = "database"`), anything else 500.
- Built: `health/` (`ping`), `job/` (insert, claim, mark done/dead, requeue, release expired leases, delete old done jobs).
- From M1-D3, `database::Database` (`begin()` → `Tx`, `connection()` → a pooled `Conn`) is how services get connections, and the new repositories take `&mut PgConnection` in **every** method: the service decides what commits together (`&mut tx`) or runs alone (`&mut conn`). Built: `user/` (find by id or email, password hash, insert guest or account, confirm a guest's email, move a guest's rows, delete, advisory lock per email), `session/` (insert, find by refresh hash current or previous, conditional rotate, revoke, revoke others, delete ended), `email_link/` (insert, count recent per address, set token, consume, retire the others, delete old), `passkey/` (challenges: insert, take once, delete expired; passkeys: insert unless known, find by credential id, ids per user, record use), `audit/` (insert with the admin's email copied, finish once). Built (M1-D4): `capture/` (insert a pending capture, photos due for the purge, forget a photo's key if it still names it; M1-D5: find, `lock` with `FOR UPDATE`, `save` the fields that change); `entitlement/` (M1-D7: lock the user's row, a design's unlock, save or renew an unlock, credit balance, spend a credit, whether Pro is active), `export/` (M1-D7: insert, find with the design's owner, finish with files or fail while pending, a design's page of summaries and its count), `design/` (M1-D6: insert, find, lock, update, a page of summaries and its count: the sort column and direction are parameters picked by `CASE`, since the SQL is fixed; search is `strpos(lower(name), lower($q))`, so no `LIKE` escaping); `job/` gained `insert_unless_recent` (the schedule check, under an advisory lock per kind).

### 4.5 `src/service`
Business logic layer — one folder per Paccradle domain.

```text
service/
├── mod.rs
├── identity/                # guests, email links, passwords, /me (M1-D3)
│   ├── mod.rs
│   ├── identity_service.rs
│   └── identity_service_trait.rs
├── session/                 # access tokens, rotating refresh tokens, guest merge at sign-in (M1-D3)
├── passkey/                 # WebAuthn ceremonies, both surfaces (M1-D3)
├── audit/                   # the admin write log (M1-D3)
├── capture/                 # uploads (M1-D4); vision runs, reading a capture, corrections (M1-D5; calls vision)
├── design/                  # create, get, versioned update, "My designs" (M1-D6; checks specs with insert_core); share links (M2)
├── export/                  # export requests, the run_export job, signed URLs (M1-D7; calls insert_core exporters)
├── billing/                 # the export entitlement and its 402 checkout (M1-D7); Paddle webhooks, purchases, refunds, portal (M1-F)
└── privacy/                 # photo purge (M1-D4), account and data deletion
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
├── http.rs                         # the HTTPS client providers share: reqwest on rustls + ring, webpki roots, no redirects (M1-D4)
├── storage/                        # built in M1-D4
│   ├── mod.rs
│   ├── storage_trait.rs            # ObjectStorage: put, get, delete (idempotent), signed_url (made locally, ≤ 7 days)
│   ├── cloudflare_r2.rs            # S3 API, path-style URLs, SigV4 (shared::sigv4); outages 503 `storage`, refusals 500
│   └── memory.rs                   # tests, and development without STORAGE_* (lost at restart); can be made to fail
├── email/                          # built in M1-D3
│   ├── mod.rs
│   ├── email_sender_trait.rs       # EmailSender, EmailMessage (plain text; Debug hides the body)
│   └── smtp.rs                     # lettre over SMTP: STARTTLS, TLS, or none (Mailpit only)
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
├── common.rs                # error body (M1-D1); PageResponse<T> + page_request (M1-D6); money later
├── meta.rs                  # HealthResponse, VersionResponse (M1-D1)
├── identity.rs              # email link, password, SessionResponse, AccountResponse (M1-D3)
├── passkey.rs               # W3C options/credential JSON, PasskeyResponse (M1-D3)
├── capture.rs               # CreateCaptureRequest + CaptureCreatedResponse (M1-D4); CaptureResponse (CLAUDE.md §5.5 JSON), CorrectionRequest (D5)
├── design.rs                # CreateDesignRequest, UpdateDesignRequest, DesignListParams, DesignResponse, DesignSummaryResponse (M1-D6)
├── export.rs                # CreateExportRequest, ExportListParams, ExportResponse, ExportSummaryResponse (M1-D7; the 402's checkout is the error's meta)
└── webhooks.rs              # Paddle webhook payloads
```

Responsibilities: request DTOs, response DTOs, HTTP request validation (validator), mappers from models to responses.

### 4.8 `src/handlers`
HTTP layer, split by **surface** (who calls), then by domain. Each surface has its own router, URL prefix, CORS origin, sign-in and semver contract version (`shared/api_surface.rs`).

| Surface | Prefix | Caller | Auth | CORS origin |
|---|---|---|---|---|
| client | `/api/client/v1` | the web app's users | bearer access token (`AuthUser` extractor, M1-D3); `/auth/…` and some routes public (share links, `version`) | `APP_WEB_URL` |
| admin | `/api/admin/v1` | the back office | **every path** behind `require_admin` (an admin's passkey session), except `/auth/…` (passkey sign-in, refresh, sign-out); fails closed (401) | `ADMIN_WEB_URL` (none when unset) |
| webhooks | `/api/webhooks/v1` | Paddle (M1-F) | signature on the raw body, no session, no CORS | none |
| probes | `/healthz`, `/readyz` | deploy scripts, uptime checks | none | none |

```text
handlers/
├── mod.rs                   # router(): probes + nest(client prefix) + nest(admin prefix) + ROUTE_NOT_FOUND fallback
├── ops/                     # /healthz, /readyz (readyz pings Postgres from M1-D2)
├── meta/                    # GET /version on each surface
├── auth/                    # /auth/{passkey/sign-in[/options],refresh,sign-out} on each surface; refresh cookie (M1-D3)
├── client/
│   ├── mod.rs               # client router: its *_route.rs files, api-version header, CORS for APP_WEB_URL
│   ├── capture/             # POST /captures: multipart upload, own body limit and hourly per-address limit (M1-D4); GET /captures/{id}, POST /captures/{id}/correct, no-store (M1-D5)
│   ├── design/              # POST/GET /designs, GET/PATCH /designs/{id} (M1-D6)
│   ├── export/              # POST/GET /designs/{id}/exports, GET /exports/{id}, no-store (M1-D7)
│   ├── billing/             # customer portal link
│   ├── identity/            # /auth/{guest,email-link[/sign-in],password/sign-in}, /me, /me/password, /me/passkeys (M1-D3)
│   └── share/               # public read-only share data
├── admin/
│   ├── mod.rs               # admin router: require_admin, api-version header, CORS for ADMIN_WEB_URL
│   ├── identity/            # GET /me (M1-D3)
│   └── <domain>/            # back-office routes (users, credits and refunds, jobs, accuracy reviews)
└── webhooks/                # Paddle: verify raw body, store event, return 200 (M1-F)
```

Each domain folder follows the same pattern: `mod.rs`, `<domain>_handler.rs`, `<domain>_route.rs`. Client and admin handlers of the same domain call the same service trait; the service checks what each caller may do (a user: their own rows; an admin: any, and every admin write is logged).

**URL naming:** lower case; plural nouns for collections (`/designs`, `/designs/{design_id}/exports`); a verb segment only for actions that aren't plain CRUD (`/captures/{capture_id}/correct`); JSON fields and query parameters in `snake_case`.

**Versioning (semver per surface):**
- The URL carries the major version; every response from a surface carries the full version in the `api-version` header. `GET /api/{surface}/v{major}/version` returns it too.
- **Patch:** a fix with no shape change. **Minor:** something old callers can ignore (an endpoint, an optional request field, a response field, an error code). **Major:** anything that can break a caller; it gets a new prefix (`/v2`) mounted next to the old one, which keeps running until its callers move (then `Deprecation`/`Sunset` headers on the old one).
- Until the public launch (M2), versions are `1.0.0-beta.N`: only our web app calls the API, so the contract may still change; each change bumps `N`. Launch freezes the client surface at `1.0.0`.
- Every contract change bumps the surface's version in `ApiSurface::version` and gets a dated `postman/` file (`code-style.md`).
- The release version (`Cargo.toml`, shown by `/healthz`) is separate: it versions the deployable, as `DEPLOYMENT.md` describes.

Responsibilities: parse requests, headers and auth extractors; validate the request schema; call services; return JSON responses and HTTP status codes. The caller's language is already in the request's extensions (`Locale`).

Rules: minimal logic; never call repositories, engines or third-party modules directly. A route belongs to exactly one surface; never mount a back-office route on the client surface "for now".

### 4.9 `src/jobs`
Background worker that processes the Postgres job table.

```text
jobs/
├── mod.rs                   # handlers(): the JobHandlers this release runs
├── handler.rs               # JobHandler (typed payload), JobHandlers (one per kind)
├── worker.rs                # claim loops + housekeeping (M1-D2)
├── send_email_link.rs       # makes a link's token and emails it, through the identity service (M1-D3)
├── purge_photos.rs          # daily: calls privacy service (photos older than 30 days, M1-D4)
├── run_vision.rs            # calls capture service; marks a run failed when its job fails for good (M1-D5)
├── run_export.rs            # calls export service; marks the export failed when its job fails for good (M1-D7)
├── process_webhooks.rs      # calls billing service for stored Paddle events (M1-F3)
└── reconcile_billing.rs     # daily: calls billing service to repair subscription drift
```

Rules: jobs behave like handlers for background work — they only call services; jobs are idempotent and safe to retry.

How the queue works (M1-D2; the states are drawn in `migrations/…_jobs.sql`):
- **Adding a job** (the outbox): a service calls `JobService::enqueue(&mut tx, &NewJob::of::<K>(&payload)?)` inside its own transaction, so the job exists exactly when its writes do.
- **A kind** is a `JobKind` in `models/job.rs` (name, payload type, `MAX_ATTEMPTS`, default 5); its handler implements `JobHandler` and is added in `jobs::handlers()`. Never rename a kind that may be queued.
- **Claiming**: `JOB_WORKERS` loops (default 2) take the oldest due job of a kind this release has a handler for (`FOR UPDATE SKIP LOCKED`), polling every 500 ms when idle. A kind added by a newer release waits through a rollback.
- **Lease**: a claim holds the job for 5 minutes. Housekeeping (every minute) requeues a job whose worker died, or buries it when it was the last attempt. A result that arrives after the lease is dropped (`JobOutcome::Lost`), so handlers must be safe to run twice.
- **Failures**: `JobError::Retry` waits 10 s, 20 s, 40 s… (at most 1 h) until the attempts run out, then the job is dead; `JobError::GiveUp` (and a payload that can't be read) is dead at once; a panic counts as a retry. Dead jobs stay for the back office; done jobs are deleted after 7 days.
- **Shutdown**: SIGTERM stops new claims; running jobs get up to 25 s to finish (the deploy allows 35 s), else their lease frees them.
- **Failed for good** (M1-D5): `JobHandler::failed_for_good(payload)` runs once a job gives up or its last attempt fails (`JobHandlers::run` knows the attempts), so a handler whose job leaves a row waiting can record it. Only `run_vision` uses it (the capture's run becomes `failed`, `CAPTURE_PROCESSING_FAILED`). A worker that dies during the last attempt skips it (the lease expires instead).
- **Schedules** (M1-D4): `jobs::schedules()` lists the kinds that run by themselves (`Schedule::of::<K>((payload, every))`). Housekeeping (at start, then every minute) adds the next run when none of the kind is queued or running and the last finished (done or dead) at least `every` ago, under an advisory lock so two processes can't both add it. Only kinds with a handler are scheduled. Built: `purge_photos` daily (10 attempts, so a storage outage of up to about 1.5 h is retried; a run longer than 3 minutes queues the next to carry on).

### 4.10 `src/middleware`
Cross-cutting HTTP concerns. `middleware::stack` wraps every route, outermost first:

1. drop a caller's `x-request-id`, then set a UUID v7 one (tower-http `SetRequestId`);
2. trace (`http{method path request_id}` span, one INFO line per response with status and latency; no query strings, headers or bodies);
3. return `x-request-id` (`PropagateRequestId`);
4. compress (gzip, brotli), **except responses that set a cookie** (BREACH: Nginx has `gzip off` for this reason);
5. `error_body`: localized JSON error bodies (§4.1);
6. catch panics → 500;
7. `timeout`: `HTTP_REQUEST_TIMEOUT_SECONDS` (30) → 503 `REQUEST_TIMEOUT`;
8. `rate_limit`: `RATE_LIMIT_PER_MINUTE` (600) per client address (`client_ip`: the `RATE_LIMIT_CLIENT_IP_HEADER` header Nginx overwrites, else the peer) → 429 `RATE_LIMITED` with `retry-after`;
9. body limit: `HTTP_BODY_LIMIT_BYTES` (1 MiB) through `DefaultBodyLimit`, which a route can raise for itself: `POST /captures` takes 15 MiB + 64 KiB (M1-D4) and keeps the 30-second timeout, since Nginx buffers the whole upload before passing it on.

Per surface (in `handlers`): `cors::for_origin`, the `api-version` header, and an `Extension(ApiSurface)` so extractors know the surface. On `/auth/…`: `same_origin::require` (an `Origin` other than the surface's web origin is `FORBIDDEN`), `no-store`, and on the sign-in routes `rate_limit::PerClient::with_rate(RATE_LIMIT_AUTH_PER_MINUTE)`. On `POST /captures`: `rate_limit::PerClient::with_window` allows 4 × `RATE_LIMIT_UPLOADS_PER_HOUR` per client address per hour, checked before the body is read (the per-user limit is in the capture service). On the admin router, every path but `/auth/…`: `admin_guard::require_admin`, which checks an admin access token and records every write in `admin_audit_log` before running it (a write it can't record doesn't run).

Who a request comes from (M1-D3): handlers take `AuthUser` (`middleware::auth`), read from `Authorization: Bearer` and checked for the surface's audience; `AUTH_REQUIRED` otherwise. Compression (item 4) also skips `Cache-Control: no-store` responses, which every token- or account-carrying response is.

### 4.11 `src/shared`
Reusable utilities with no business logic.

Built (M1-D1): `api_surface.rs` (surfaces, prefixes, contract versions), `id.rs` (UUID v7), `logger.rs` (JSON logs in production), `rate_limiter.rs` (in-memory token bucket per key, idle keys swept). Built (M1-D3): `base64url.rs` (canonical, unpadded), `crypto.rs` (secure random, SHA-256, `NewToken` + `token_hash`), `jwt.rs` (HS256 only, one fixed header, constant-time check), `password.rs` (Argon2id PHC, `DUMMY_HASH` for equal timing), `webauthn/` (CBOR reader, COSE keys ES256/EdDSA/RS256, authenticator and client data, `verify_registration`, `verify_authentication`). Built (M1-D4): `sigv4.rs` (AWS Signature Version 4 for the S3 API: signed headers and presigned URLs, checked against AWS's worked examples); `rate_limiter.rs` gained `with_window` (hourly upload limits). Later: money helpers (minor units, formatting), unit helpers (mm), pagination helpers.

## 5. Data model (core tables)
See CLAUDE.md §8.4 for the full list, and `migrations/` for the columns and rules:
`users`, `captures`, `products`, `designs`, `design_unlocks`, `credit_ledger`, `subscriptions`, `paddle_events`, `exports`, `jobs` (M1-D2), plus the sign-in tables of M1-D3: `users.role`, `email_confirmed_at`, `password_hash`; `sessions`, `email_links`, `passkeys`, `passkey_challenges`, `admin_audit_log`. M1-D5 added `captures.run` (the run number) and `captures.error_meta` (the stored error's values).

Conventions (the header of the first migration):
- IDs are UUID v7 made by the API (`shared::id::new_id`), never serial numbers; external IDs (Paddle's) are unique text columns next to them.
- Times are `timestamptz`; `updated_at` is kept by the `set_updated_at()` trigger.
- Enum-like text columns hold lower-case snake_case values with a CHECK constraint; adding a value is a new migration.
- Rows belong to a user and go with the user (`ON DELETE CASCADE`), so deleting an account deletes its data (§7).
- A user without an email is a guest; signing in puts the email on the same row. Emails are unique whatever their case, and stored only once confirmed (`users_email_confirmed`).
- `admin_audit_log` only grows (a trigger): a row's status is written once, and the admin link is cleared, not the row, when the account goes.
- `credit_ledger` is append-only (a trigger refuses updates); its balance is `SUM(delta)`, and a pack's credits are added once per Paddle transaction.
- Migrations only add (new tables, columns, constraints that existing rows pass), so the previous release still runs after a rollback. Removing something waits for a release that no longer uses it.

## 6. Integration rules
- **Paddle webhooks:** `POST /api/webhooks/v1/paddle`. The handler takes the **raw body**, the service verifies `Paddle-Signature` (HMAC-SHA256 over `ts:body`, constant-time compare, reject old timestamps), stores the event in `paddle_events` keyed by `event_id`, returns 200 quickly, and a job processes it. Processing must be idempotent (CLAUDE.md §9.4).
- **Entitlements:** export requests check Pro → credits → otherwise return 402 with the checkout payload (CLAUDE.md §9.3). Production vector files are only generated on the server after this check (CLAUDE.md §7.4).
- **Outgoing side effects** (emails, vision runs, exports, AI calls) go through the job table, never inline in request handlers.

## 7. Security & privacy rules
- **Uploads:** 15 MB maximum (`CAPTURE_FILE_TOO_LARGE`), file type from the bytes and pixel count from the header before anything is decoded or stored (`photo_file::inspect`; decompression bombs), 30 an hour per user and 4 times that per client address (`RATE_LIMIT_UPLOADS_PER_HOUR`) (CLAUDE.md §8.5).
- **Photos:** stored as `photos/{capture_id}.{ext}`, private R2 objects, after `photo_file::clean` keeps only the picture, its colour profile and its EXIF orientation (no GPS or other EXIF, XMP, comments, thumbnails, or anything after the image); the camera make, model and focal length the browser read go to `captures.exif`, any other field it sends (a location included) is dropped unread. Deleted after 30 days by the daily purge; keep only outlines and sizes. A photo stored for a capture that then fails to save is deleted again. Photo bytes never reach a log line (`NewObject` and `PhotoUpload` print sizes only). Photos are only sent to an AI provider when the user hasn't opted out (CLAUDE.md §5.4).
- **Paid files:** short-lived signed URLs only.
- Refresh tokens issued as `HttpOnly; Secure; SameSite=Strict` cookies scoped to the surface's `/auth` path on the API host; CORS allows only the app's own origin. The web app and the API must be on the same site (`app.` and `api.paccradle.com`), or browsers drop the cookie.
- **Back office:** the admin surface fails closed: every admin path, unknown ones included, answers 401 unless the caller is a signed-in admin, so admin routes can't be discovered; only `/auth/…` is open. Its CORS origin (`ADMIN_WEB_URL`) is separate from the web app's. Admins sign in with a passkey only (no password, no email link), get 5-minute access tokens and a 12-hour session that refreshes don't extend; losing the role ends the session at its next refresh. Every admin write is logged (who, what, when, status) before it runs. The role is granted in SQL for now.
- Never compress a response that carries a secret next to caller-chosen text (BREACH): responses that set a cookie are sent uncompressed.
- Support data export and account deletion for users.
- Secrets only from environment/secret store; never committed.
- No user-supplied code ever runs on the server.

## 8. Sign-in (M1-D3, DEC-07)
- **Accounts.** A visitor starts as a guest (`POST /auth/guest`: a `users` row without email). `POST /auth/email-link` sends one email either way: "confirm your email" (no account; link valid 24 h) or a sign-in link (account; 15 min); the answer never says which. The link (`{APP_WEB_URL}/auth/link#token=…`) confirms the address and signs in: it makes the account (from the browser's guest, which keeps its rows, else a new row) or signs in to the existing one (a guest's captures, products and designs move over and the guest goes). Using one link retires the address's other links. At most 3 links per address per 10 minutes.
- **Then.** The session answer carries `next_steps`: `set_password` (no password yet) and `add_passkey` (the sign-in didn't use one; the web app offers it only where the device supports passkeys). A password (12–128 characters, Argon2id) may be set without the current one within 15 minutes of a sign-in by link or passkey (that's "forgot password"); otherwise the current one is needed. Setting it ends the user's other sessions.
- **Signing in later.** Passkey first (discoverable, user verification required); the web app falls back to the password form and "email me a link" after 2 failed tries. Password sign-ins answer every mismatch with the same `AUTH_INVALID_CREDENTIALS` in the same time (a dummy Argon2 hash) and allow 5 tries per address per minute.
- **Sessions.** Access token: HS256 JWT (`sub`, `sid`, `aud` = surface, `role`, 15 min; 5 on the back office). Refresh token: 256 random bits in the cookie, SHA-256 in `sessions`, replaced on every refresh; the client session slides to 30 days. A refresh token reused within 30 s of its replacement (two tabs) gets an access token and no new cookie; later, it revokes the session. Any sign-in revokes the browser's previous session.
- **Passkeys.** Options and answers are the W3C JSON shapes. Registration needs a signed-in account; attestation is not checked (`none`). Challenges are single-use, 5 minutes, and deleted before the answer is checked, so a refused answer can't be retried. Every refusal is `AUTH_PASSKEY_REJECTED`; the reason is logged. The passkey's user handle is the user's id. `WEBAUTHN_RP_ID` must cover the web app's and the back office's hosts and never change after launch.
- **Housekeeping.** Expired sessions (30 days after expiry), old links (7 days) and expired challenges are deleted when new ones are made; there's no scheduled job yet.

## 9. Captures and vision runs (M1-D5)
- **A capture's run.** `captures.run` numbers its vision runs (1 at upload). A tap starts the next one (status `pending`, the last outline kept), and so does a height typed while a run is pending; an edit moves the number on too, since it outdates a pending run. Every change to a capture happens under its row lock (`CaptureRepository::lock`) in one transaction, with the next run's `run_vision` job queued in the same transaction.
- **The job** (`{capture_id, run}`) reads the capture without a lock, skips it when the run is no longer the capture's or it isn't pending, reads the photo from storage (gone: `CAPTURE_PHOTO_DELETED`), runs `vision::run` in a blocking task (two at a time per process, `VISION_AT_ONCE`), then saves under the lock only if the run is still the capture's. Whatever vision says is saved (status `failed` with `error_code` and `error_meta`; `CAPTURE_PROCESSING_FAILED`, an OpenCV error, is also logged); storage and database outages are errors the job retries.
- **Corrections** (`POST /captures/{id}/correct`, exactly one of): `tap_point_px` → 202, a new run (counts against the hourly vision runs; `CAPTURE_PHOTO_DELETED` once the photo is gone); `edited_contour_px` → 200, `vision::accept_edit` at once (`CAPTURE_NOT_READY` without an outline, `CAPTURE_OUTLINE_CROSSES_ITSELF`); `height_mm` → 200 with `vision::set_height` applied, or 202 with a new run while one is pending.
- **Reading** (`GET /captures/{id}`): status, settings, the latest outline (the §5.5 JSON, kept while a newer run is pending or after it failed), the last error in the caller's language, and a 10-minute signed photo link while the photo is kept. Someone else's capture is `CAPTURE_NOT_FOUND`, as is an unknown id. Both answers are `no-store`.
- **Codes.** Every `vision` code is an `ErrorCode` (`ErrorCode::from_code` reads a stored one back; an unknown one reads as `CAPTURE_PROCESSING_FAILED`). Vision's `InvalidInput` names the setting (`focal_length_35mm` is `exif.focal_length_35mm`, as the upload sends it) and what it allows.

## 10. Designs (M1-D6)
- **A design** is the editor's `insert_core::Design` (`designs.spec`) with a name (≤ 200 characters, may be empty) and a version. Its products carry **copies** of their outlines (from a capture's `result.products[].contour_mm`, or typed): a capture corrected later doesn't change a saved design. The `products` table stays unused until a product library or the kit builder (M3) needs products across designs.
- **Saving** checks the spec twice: its shape when it is read (`DESIGN_INVALID` with serde's words) and its rules in the design service (`Design::validate`, `DESIGN_INVALID` with `insert_core`'s words in `meta.detail`). The messages are English: the editor runs the same rules in WASM, so these mean a bug or a hand-made request. A design that can't be generated yet (it doesn't fit its box…) is still saved; exports (M1-D7) generate it.
- **Versions.** `POST` saves version 1. `PATCH` carries the version it was made from: another one is `DESIGN_VERSION_CONFLICT` with `meta.current_version` (another tab saved first; the editor reloads). A change that changes nothing keeps the version, so autosaving the same state costs nothing. Every saved change adds one; an export records the version it was made from.
- **"My designs"** (`GET /designs`): `page` (from 1), `page_size` (1–100, default 20), `search` (name, any case), `style`, `sort` (`updated_at` default, `created_at`, `name`), `order` (default `asc` by name, `desc` by date); `{items, page, page_size, total, total_pages}`. Items are summaries without outlines (style, product types and count, whether it has a box). Unknown parameters are `INVALID_REQUEST`.
- **Privacy.** Someone else's design, or an unknown id, is `DESIGN_NOT_FOUND`. Guests save designs too; they move to the account when the guest signs up or in (M1-D3).

## 11. Exports and entitlements (M1-D7, CLAUDE.md §9.3)
- **Asking** (`POST /designs/{id}/exports {formats, paper?, version?}`): an account only (`AUTH_ACCOUNT_REQUIRED` for a guest, DEC-14); 60 an hour per user; the design must be the caller's and, when `version` is sent, current (`DESIGN_VERSION_CONFLICT`). The design is generated first: one that can't be made yet is `DESIGN_NOT_BUILDABLE` (422, `insert_core`'s words) and costs nothing.
- **Free and paid.** `png` (preview) and `test_print` (needs `paper`) are free. `svg`, `pdf`, `dxf` need `BillingService::entitle_export`, run in the export's own transaction with the user's row locked (`SELECT … FOR UPDATE`, so two clicks spend one credit): an unlock whose 30-day edit window covers this version (the design's `updated_at` ≤ `edits_until`) → `unlocked`; Pro (`active`, `trialing`, `past_due`) → `pro`, unlocking the design for 30 days; a credit (`SUM(delta)` > 0) → `credit`, a `spend` row of −1 and the unlock; otherwise `EXPORT_PAYMENT_REQUIRED` (402) with `meta.checkout` (`price_id` of a single export, `custom_data {user_id, design_id}`, `customer_email`) and `meta.offers` (the four price ids). Prices come from `PADDLE_PRICE_*` (required in production).
- **The export** keeps a copy of the spec, the design's name and version, the paper and `paid_with`, and its `run_export` job is queued in the same transaction. The job makes every file from that copy, stores it at `exports/{id}/{file name}` with `Content-Disposition: attachment; filename="paccradle-<name>-v<version>…"`, then marks the export done; a file that can't be made marks it failed (logged); storage outages retry; a job that fails for good marks it `EXPORT_FAILED`. Asking again is free: the design stays unlocked.
- **Downloading** (`GET /exports/{id}`): files with 5-minute signed links once done (`EXPORT_URL_TTL`); someone else's export is `EXPORT_NOT_FOUND`. History: `GET /designs/{id}/exports` (paged, `status` filter, newest first). Export files are kept: re-downloading never expires.

## Related files
- `CLAUDE.md` (repo root) — product plan
- `MILESTONES.md` (repo root) — tasks, decisions, progress
- `backend-tech-stack.md` — backend languages, libraries and coding rules
- `backend-architecture.md` — backend layers, folder structure, data model, integrations and security (this file)
- `code-style.md` — coding style

## Build app

always make our app when we build image and run container should be smaller/lightweight image and secure
