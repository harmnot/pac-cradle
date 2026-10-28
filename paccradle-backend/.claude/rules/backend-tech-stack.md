# Backend Tech Stack — Paccradle

> Backend technology choices and coding rules for AI assistants. Product plan: `CLAUDE.md` at the repo root. Backend design: `backend-architecture.md`. Style: `code-style.md`.

## 1. Stack (Rust + Axum)
| Concern         | Choice                                                                                                                         |
|-----------------|--------------------------------------------------------------------------------------------------------------------------------|
| Language        | Rust (stable toolchain, edition 2024), one Cargo workspace: API in `src/`, engines in `crates/`                                |
| Web framework   | Axum on Tokio                                                                                                                  |
| Middleware      | tower / tower-http (trace, CORS, compression, body limit, timeout, request-id, rate limit)                                      |
| Database        | PostgreSQL                                                                                                                     |
| DB access       | SQLx (compile-time checked queries, `sqlx migrate` for migrations) — no ORM                                                    |
| Serialization   | serde / serde_json                                                                                                             |
| Validation      | validator (DTO validation at the handler boundary)                                                                             |
| Errors          | thiserror for domain errors, anyhow only in binaries/scripts (e.g. the vision spike CLI)                                       |
| Logging/tracing | tracing + tracing-subscriber (JSON logs in production)                                                                         |
| HTTP client     | reqwest (Paddle API, email provider, AI provider later)                                                                        |
| Auth            | Email magic link (DEC-07); short-lived JWT access token + rotating refresh token, both stored hashed; no passwords             |
| Config          | Environment variables (dotenvy for local), loaded once into a typed `AppConfig`                                                |
| Background jobs | Postgres job table (`FOR UPDATE SKIP LOCKED`), worker in the same binary at first                                              |
| File storage    | S3-compatible object storage (Cloudflare R2); signed URLs only                                                                 |
| Vision          | `opencv` crate (needs system OpenCV + libclang); `ort` for SAM 2 later (pin the exact version, CLAUDE.md §5.4)                 |
| Geometry        | `insert_core`: `kurbo` for paths, a polygon offset/boolean library (`i_overlay` or Clipper2, picked in M1-A2)                  |
| Exports         | SVG, PDF (`printpdf` or `krilla`), DXF (`dxf`) — server-only feature of `insert_core`                                          |
| WASM            | `wasm-bindgen` build of `insert_core` for the browser preview (no exporters)                                                   |
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

## 3. External services
| Purpose        | Service                                                                    |
|----------------|----------------------------------------------------------------------------|
| Payments       | Paddle (merchant of record) behind `BillingProvider`; Plan B MoRs in CLAUDE.md §9.6 — no Stripe |
| Email          | Transactional email provider (magic links)                                 |
| AI (v1)        | L3 vision helper behind a trait: Hugging Face → Gemini free tier → fal.ai (CLAUDE.md §5.4, DEC-13). No Claude API |
| Object storage | Cloudflare R2                                                              |

## Related files
- `CLAUDE.md` (repo root) — product plan
- `MILESTONES.md` (repo root) — tasks, decisions, progress
- `backend-tech-stack.md` — backend languages, libraries and coding rules (this file)
- `backend-architecture.md` — backend design, data model, integrations and security
- `code-style.md` — coding style
