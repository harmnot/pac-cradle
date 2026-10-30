---
name: dev-seed
description: The fixed development accounts for Paccradle (an admin, a buyer who still has to pay for exports, a buyer with credits, a Pro subscriber), always the same names, emails and password, and how to seed, reset and sign in as them. Use whenever a task needs seed data, test users, test credentials, a signed-in account or admin, a manual or real run against the dev API (curl, Postman, the web app, the back office), or checking exports, the 402 checkout, credits, Pro or admin routes by hand. Never invent other test accounts or passwords; use or extend these.
---

# Dev seed accounts

The same four accounts every time, so the person who owns this project can sign in later and check what was built. **Never make up other test users, emails or passwords for manual checks; use these.** (Automated tests in `paccradle-backend/tests/` make their own throwaway accounts on throwaway databases; that's fine and separate.)

## The accounts

Password for every one: **`paccradle-dev-pass`**

| Name | Email | Role | What it has after seeding |
|---|---|---|---|
| Ada Admin | `admin@paccradle.test` | admin | The admin role, a password for the web app, and a seeded passkey for the back office. No designs. |
| Bima Buyer | `buyer@paccradle.test` | user | Free plan, **0 credits**, 3 designs (perfume cradle, lipstick card, candle jar grid). SVG/PDF/DXF exports answer **402** `EXPORT_PAYMENT_REQUIRED` with the checkout: the "wants to buy the export file" path. PNG and the test print are free. |
| Citra Credits | `credits@paccradle.test` | user | Bought a 5-pack, spent 1 on her perfume cradle (unlocked for 30 days, one export queued): **4 credits** left. Her lipstick card isn't unlocked, so exporting it spends a credit. |
| Dewi Pro | `pro@paccradle.test` | user | **Pro monthly, active** for 30 days from seeding (`sub_dev_seed_dewi_pro`). One candle jar grid. Exports don't spend credits. Paper default: Letter. |

Fixed ids (stable across re-seeds):

| What | Id |
|---|---|
| Ada / Bima / Citra / Dewi | `019a0000-5eed-7000-8000-0000000000a1` / `…00b1` / `…00c1` / `…00d1` |
| Bima's cradle, card, grid | `019a0000-5eed-7000-8001-0000000000b1`, `…00b2`, `…00b3` |
| Citra's cradle (unlocked), card | `019a0000-5eed-7000-8001-0000000000c1`, `…00c2` |
| Dewi's grid | `019a0000-5eed-7000-8001-0000000000d1` |

The users table has no name column yet, so the names show in design names ("Bima's perfume cradle") and here, not in `/me`.

## Seed or reset

From `paccradle-backend/`, with the dev database up (`scripts/dev-db.sh`) and the toolchain image built:

```bash
scripts/dev-seed.sh               # (re)creates the four accounts exactly as above
```

- It deletes those four accounts (by id, and any account using their emails) with everything they own, then recreates them. Nothing else in the database is touched. Run it again any time to undo what a check changed (a spent credit, an unlocked design, a new password).
- It refuses to run unless `APP_ENV=development` and the database host is this machine. Never point it at production: the password and the admin passkey's private key are in the repo.
- Citra's export is queued as a job; the API's worker (`scripts/dev-api.sh`) makes its files.

## Sign in

**Web app accounts** (any of the four), against the running API (`scripts/dev-api.sh`, http://127.0.0.1:7773):

```bash
curl -s http://127.0.0.1:7773/api/client/v1/auth/password/sign-in \
  -H 'content-type: application/json' \
  -d '{"email":"buyer@paccradle.test","password":"paccradle-dev-pass"}'
# → {"access_token": …}; then -H "Authorization: Bearer <access_token>"
```

**Back office** (Ada only; admins sign in with a passkey, never a password):

```bash
scripts/dev-seed.sh admin-token
```

It signs Ada in with the seeded passkey through the API's real passkey sign-in, swaps the session at the running API for an access token that API accepts (5 minutes), and prints that token plus the refresh cookie (the session lasts 12 hours after sign-in). Without `AUTH_JWT_SECRET` in `.env`, each API start signs tokens with its own random key, which is why the token comes from the running API. If the API isn't running, it prints only the cookie. `DEV_API_URL` overrides where it looks (as the container sees it; default `http://host.docker.internal:<APP_PORT>`).

**Postman**: import `paccradle-backend/postman/dev-seed-2026-10-30.postman_environment.json` and `…postman_collection.json`. "Sign in as …" saves `access_token`, `refresh_token` and `design_id` for every other collection. For the back office, paste the cookie value from `admin-token` into `admin_refresh_token` and send "Back office: refresh".

**In a browser** (M1-E1): run `scripts/dev-api.sh` and `pnpm dev` in `paccradle-frontend-app/` (`NEXT_PUBLIC_API_URL=http://localhost:7773`: `localhost`, not `127.0.0.1`), open http://localhost:3000/sign-in, choose "Continue with password" and use the email and password above. The app offers a passkey right after (the accounts have none): "Not now" skips it; a passkey you add stays on the account until the next seed. Email links go to Mailpit (http://localhost:8025). For the back office in a real browser, add a passkey as Ada in the web app (the seeded one lives only in `dev_seed.rs`); the admin surface also needs `ADMIN_WEB_URL` (e.g. `http://localhost:3001`) in `.env`.

## Changing the accounts

The source of truth is `paccradle-backend/examples/dev_seed.rs` (`PERSONAS`, `DESIGNS`). When a new kind of account is needed (e.g. a canceled subscription, a refunded purchase), add a persona there with a fixed id and a `@paccradle.test` email, then update in the same change: this file's tables, the Postman environment and collection (`postman/dev-seed-<date>.postman_*`), and the MILESTONES progress log. Keep existing personas' names, emails, ids and password unchanged: the project owner relies on them.

After changing its queries, refresh the sqlx cache: `scripts/in-docker.sh cargo sqlx prepare -- --all-targets`.
