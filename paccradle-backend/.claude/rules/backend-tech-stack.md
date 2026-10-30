# Backend Tech Stack — Paccradle

> Backend technology choices and coding rules for AI assistants. Product plan: `CLAUDE.md` at the repo root. Backend design: `backend-architecture.md`. Style: `code-style.md`.

## 1. Stack (Rust + Axum)
| Concern         | Choice                                                                                                                         |
|-----------------|--------------------------------------------------------------------------------------------------------------------------------|
| Language        | Rust (stable toolchain, edition 2024), one Cargo workspace: API in `src/`, engines in `crates/`                                |
| Web framework   | Axum on Tokio                                                                                                                  |
| Middleware      | tower-http (request-id, trace, CORS, compression, set-header, catch-panic); own small middleware for the error body, timeout and per-IP rate limit (M1-D1) |
| Database        | PostgreSQL                                                                                                                     |
| DB access       | SQLx 0.9 (compile-time checked `query!` macros with the committed `.sqlx/` cache for offline builds; embedded migrations applied at startup) — no ORM. Local Postgres 18: `scripts/dev-db.sh` |
| Serialization   | serde / serde_json                                                                                                             |
| Validation      | validator (DTO validation at the handler boundary)                                                                             |
| Errors          | thiserror for domain errors, anyhow only in binaries/scripts (e.g. the vision spike CLI)                                       |
| Logging/tracing | tracing + tracing-subscriber (JSON logs in production)                                                                         |
| HTTP client     | reqwest 0.13 (added in M1-D4 for R2; Paddle and the AI providers reuse it) through `third_party::http::client`: rustls on `ring` with `webpki-roots`, as sqlx and lettre build it (reqwest's default TLS is aws-lc-rs, a C build), HTTP/1.1, no redirects, no proxy from the environment. Email goes over SMTP (`lettre`), not HTTP |
| Auth            | Own sign-in (DEC-07, M1-D3): an email link confirms the address (sign-up, and "forgot password"), then an Argon2id password (`argon2`) and passkeys (WebAuthn checked in `shared::webauthn`: own CBOR reader, signatures on `ring`; the WebAuthn crates pull in OpenSSL). HS256 JWT access tokens (15 min, back office 5 min) + a rotating refresh token in an `HttpOnly; SameSite=Strict` cookie, stored as SHA-256. The back office signs in with a passkey only |
| Crypto          | `ring` (already built by sqlx's rustls): secure random, SHA-256, HMAC-SHA256, P-256 / Ed25519 / RSA signature checks. Own base64url (`shared::base64url`) |
| Email           | `lettre` over SMTP with rustls (any provider; SMTP2GO planned). One connection per email. Locally: Mailpit (`scripts/dev-mail.sh`) |
| Time            | `time` (sqlx `time` feature): `timestamptz` as `OffsetDateTime`, RFC 3339 in JSON (`time::serde::rfc3339`). Expiry arithmetic for new rows stays in SQL (`now() + make_interval(…)`) |
| Config          | Environment variables, then `./.env` for local runs (own reader with Docker `--env-file` rules; no dotenvy, which writes the process environment, `unsafe` in edition 2024), loaded once into a typed `AppConfig` |
| Background jobs | Postgres job table (`FOR UPDATE SKIP LOCKED`), worker in the same binary at first                                              |
| File storage    | Cloudflare R2 through its S3 API (`third_party::storage::CloudflareR2`), SigV4 signed by hand (`shared::sigv4`, HMAC and SHA-256 from `ring`; no AWS SDK); private bucket, signed URLs only. `MemoryStorage` for tests and for development without `STORAGE_*` |
| Uploads         | `axum` `multipart` feature (multer); `crates/photo_file` (pure Rust, M1-D4) checks the header and keeps only picture, colour profile and orientation, so the API needs no OpenCV to take a photo |
| Vision          | `opencv` crate (needs system OpenCV + libclang); the API links it through `vision` since M1-D5 and runs it in the `run_vision` job, two at a time. `ort` for SAM 2 later (pin the exact version, CLAUDE.md §5.4) |
| Geometry        | `insert_core`: own `Path`/`Polygon` types (lines + arcs, no `kurbo`); `i_overlay` for offsets and booleans, used only in `geometry/area.rs` |
| Exports         | `insert_core` `export` feature (server only; the API crate turns it on): one `export::Sheet` layout drawn as SVG, PDF and DXF R12, all hand-written (B1 to B3); the free print-at-home test PDF (B4, `export::TestPrint`) tiles the preview geometry over A4/Letter with the same PDF writer; the free PNG preview (B5, `export::Preview`) is drawn with `tiny-skia` + `ttf-parser` and the bundled Geist font (`crates/insert_core/fonts/`, OFL) |
| WASM            | `crates/insert_core_wasm`: `wasm-bindgen` wrapper of `insert_core` for the browser preview (JSON in/out, drawn lines as straight segments within 0.05 mm with points to 0.01 mm, no exporters). `scripts/build-wasm.sh` builds it into the frontend; the `wasm-bindgen` CLI must match the crate version pinned in its `Cargo.toml` |
| API docs        | Postman: dated contract files in `postman/` (see `code-style.md`)                                                              |
| IDs             | UUID v7                                                                                                                        |
| Time            | UTC everywhere (`timestamptz`)                                                                                                 |
| Testing         | cargo test, `#[sqlx::test]` with a real Postgres; golden SVG snapshots for `insert_core`; accuracy regression set for `vision` |
| Quality         | `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, cargo-audit in CI                                        |
| Deploy          | Docker image (`Dockerfile`: toolchain → build → runtime), single VPS at first; GitHub Actions CI                               |

## 2. Coding rules
- No `unwrap()` / `expect()` outside tests and startup code (enforced by workspace clippy lints; `clippy.toml` allows them in tests).
- Money as integer minor units (`i64`, USD cents) plus ISO currency code. Never use floats for money.
- Lengths are `f64` millimetres everywhere; convert pixels to mm at the vision boundary only.
- All writes that span tables run in a transaction.
- Every list endpoint is paginated with `page`, `page_size`, `total_pages`, plus filtering and sorting (see `code-style.md`).
- Authorization is checked in the service layer: a user only reads and changes their own captures, designs and exports.
- Without OpenCV on the Mac, run cargo inside the toolchain image: `scripts/in-docker.sh cargo test --workspace`.
- The API links OpenCV since M1-D5 (the capture job runs `vision`), so on a Mac without OpenCV every API build runs in the toolchain image: `scripts/in-docker.sh cargo test -p paccradle-backend`, `scripts/in-docker.sh cargo sqlx prepare -- --all-targets` (the image has sqlx-cli), and `scripts/dev-api.sh` to run the API on http://127.0.0.1:7773. `photo_file` and `insert_core` still build natively. Database tests need Postgres (`scripts/dev-db.sh`; each `#[sqlx::test]` gets a throwaway database with every migration); `in-docker.sh` points `DATABASE_URL` at it through `host.docker.internal:54329`.
- The runtime image carries OpenCV's core, imgproc and imgcodecs libraries (Debian `libopencv-*410`), the only ones the binary needs (`readelf -d`); they make it about 550 MB until a slimmer OpenCV build (M1-G1).
- Traits used through `Arc<dyn …>` return `shared::future::BoxFuture` instead of using `async fn` (not callable through `dyn` on Rust 1.99; no `async-trait` crate). Traits used only generically (`JobHandler`) may return `impl Future + Send`.
- Secrets never reach a log line: `shared::secret::Secret` for settings; request bodies and service inputs holding a password or token have no `Debug`; `EmailMessage`'s `Debug` shows only the subject. Tokens are stored as SHA-256, passwords as Argon2id; a link's raw token isn't even in the job queue.
- Sign-in emails locally: `scripts/dev-mail.sh` starts Mailpit (the development SMTP default). Tests swap the providers (`AppState::with_providers`: an outbox for email, `MemoryStorage` for photos). `AppState::new` builds them from the settings and fails only if the HTTPS client can't be set up.
- Dev accounts (M1-D9): `scripts/dev-seed.sh` resets four fixed accounts with one password (an admin, a buyer who must pay, a buyer with credits, a Pro subscriber); `scripts/dev-seed.sh admin-token` signs the admin in to the back office with the seeded passkey. Use these for every manual check, never new ad-hoc accounts; the list is in the repo root's `.claude/skills/dev-seed/SKILL.md`, the source in `examples/dev_seed.rs` (development and a local database only).
- Endpoints (M1-D9): `tests/endpoints.rs` keeps a table of every route and who may call it, checked against the route files, the router and the dated `postman/` contracts. A new route goes in that table and in a dated contract, or CI fails; route paths stay string literals in `.route("…", …)` so the check can read them.
- Photos locally: set `STORAGE_*` to an R2 dev bucket, or leave all four empty to keep photos in memory. `cargo test --test storage -- --ignored` runs a real round trip (put, get, signed download, delete) against the bucket in `.env`.

## 3. External services
| Purpose        | Service                                                                    |
|----------------|----------------------------------------------------------------------------|
| Payments       | Paddle (merchant of record) behind `BillingProvider`; Plan B MoRs in CLAUDE.md §9.6 — no Stripe |
| Email          | SMTP to a transactional provider (SMTP2GO planned): sign-in links. Mailpit locally |
| AI (v1)        | L3 vision helper behind a trait: Hugging Face → Gemini free tier → fal.ai (CLAUDE.md §5.4, DEC-13). No Claude API |
| Object storage | Cloudflare R2                                                              |

## Related files
- `CLAUDE.md` (repo root) — product plan
- `MILESTONES.md` (repo root) — tasks, decisions, progress
- `backend-tech-stack.md` — backend languages, libraries and coding rules (this file)
- `backend-architecture.md` — backend design, data model, integrations and security
- `code-style.md` — coding style
