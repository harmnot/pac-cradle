//! The schema's own rules (M1-D2, `migrations/`): what Postgres refuses whatever the code
//! does. Each test gets a fresh database with every migration applied.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use paccradle_backend::shared::id::new_id;
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool, SqlSafeStr};
use uuid::Uuid;

const UNIQUE_VIOLATION: &str = "23505";
const CHECK_VIOLATION: &str = "23514";
const RAISED: &str = "P0001";

/// The SQLSTATE a statement failed with.
fn sql_state<T: std::fmt::Debug>(result: Result<T, sqlx::Error>) -> String {
    let error = result.expect_err("the statement is refused");
    let database_error = error.as_database_error().expect("a database error");
    database_error.code().expect("with a SQLSTATE").into_owned()
}

/// A guest, or an account whose email is confirmed (M1-D3: only a confirmed email is stored).
async fn user(pool: &PgPool, email: Option<&str>) -> Uuid {
    let id = new_id();
    sqlx::query(
        "INSERT INTO users (id, email, email_confirmed_at) \
         VALUES ($1, $2, CASE WHEN $2::text IS NULL THEN NULL ELSE now() END)",
    )
    .bind(id)
    .bind(email)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn design(pool: &PgPool, user_id: Uuid) -> Uuid {
    let id = new_id();
    sqlx::query("INSERT INTO designs (id, user_id, spec) VALUES ($1, $2, '{}')")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn count(pool: &PgPool, sql: impl SqlSafeStr, user_id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn guests_have_no_email_and_an_address_has_one_account_whatever_its_case(pool: PgPool) {
    user(&pool, None).await;
    user(&pool, None).await;
    user(&pool, Some("Ana@Example.com")).await;

    let again = sqlx::query(
        "INSERT INTO users (id, email, email_confirmed_at) VALUES ($1, 'ana@example.com', now())",
    )
    .bind(new_id())
    .execute(&pool)
    .await;
    assert_eq!(sql_state(again), UNIQUE_VIOLATION);

    for bad in [" ana@example.com", "no-at-sign", "@example.com"] {
        let result =
            sqlx::query("INSERT INTO users (id, email, email_confirmed_at) VALUES ($1, $2, now())")
                .bind(new_id())
                .bind(bad)
                .execute(&pool)
                .await;
        assert_eq!(sql_state(result), CHECK_VIOLATION, "{bad}");
    }
    // An email is only stored once confirmed (M1-D3).
    let unconfirmed = sqlx::query("INSERT INTO users (id, email) VALUES ($1, 'bo@example.com')")
        .bind(new_id())
        .execute(&pool)
        .await;
    assert_eq!(sql_state(unconfirmed), CHECK_VIOLATION);
}

#[sqlx::test]
async fn enum_like_columns_refuse_values_the_code_doesnt_know(pool: PgPool) {
    let user_id = user(&pool, None).await;
    let design_id = design(&pool, user_id).await;

    let paper = sqlx::query("UPDATE users SET paper_default = 'a5' WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;
    assert_eq!(sql_state(paper), CHECK_VIOLATION);

    let capture = sqlx::query(
        "INSERT INTO captures (id, user_id, status, mode, paper, input, purge_after)
         VALUES ($1, $2, 'thinking', 'photo', 'a4', '{}', now())",
    )
    .bind(new_id())
    .bind(user_id)
    .execute(&pool)
    .await;
    assert_eq!(sql_state(capture), CHECK_VIOLATION);

    for formats in [vec!["cf2"], vec![]] {
        let export = sqlx::query(
            "INSERT INTO exports (id, design_id, design_version, formats, spec, paid_with) \
             VALUES ($1, $2, 1, $3, '{}', 'free')",
        )
        .bind(new_id())
        .bind(design_id)
        .bind(&formats)
        .execute(&pool)
        .await;
        assert_eq!(sql_state(export), CHECK_VIOLATION, "{formats:?}");
    }
}

#[sqlx::test]
async fn updated_at_follows_every_change(pool: PgPool) {
    let user_id = user(&pool, None).await;
    sqlx::query("SELECT pg_sleep(0.01)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET locale = 'id' WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let moved: bool = sqlx::query_scalar("SELECT updated_at > created_at FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(moved);
}

#[sqlx::test]
async fn the_credit_ledger_only_grows_and_counts_a_purchase_once(pool: PgPool) {
    let user_id = user(&pool, None).await;
    let add = |delta: i32, reason: &'static str, transaction: Option<&'static str>| {
        sqlx::query(
            "INSERT INTO credit_ledger (id, user_id, delta, reason, paddle_transaction_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_id())
        .bind(user_id)
        .bind(delta)
        .bind(reason)
        .bind(transaction)
        .execute(&pool)
    };

    add(5, "purchase", Some("txn_01")).await.unwrap();
    // Paddle resends the webhook: the pack must not be credited twice.
    assert_eq!(
        sql_state(add(5, "purchase", Some("txn_01")).await),
        UNIQUE_VIOLATION
    );
    // Spending adds credits? Buying without a checkout? Both refused.
    assert_eq!(sql_state(add(1, "spend", None).await), CHECK_VIOLATION);
    assert_eq!(sql_state(add(5, "purchase", None).await), CHECK_VIOLATION);

    let edit = sqlx::query("UPDATE credit_ledger SET delta = 50 WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;
    assert_eq!(sql_state(edit), RAISED);

    let balance = count(
        &pool,
        "SELECT sum(delta)::bigint FROM credit_ledger WHERE user_id = $1",
        user_id,
    )
    .await;
    assert_eq!(balance, 5);
}

#[sqlx::test]
async fn deleting_a_design_keeps_the_credit_it_cost(pool: PgPool) {
    let user_id = user(&pool, None).await;
    let design_id = design(&pool, user_id).await;
    sqlx::query(
        "INSERT INTO credit_ledger (id, user_id, delta, reason, design_id) VALUES ($1, $2, -1, 'spend', $3)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(design_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM designs WHERE id = $1")
        .bind(design_id)
        .execute(&pool)
        .await
        .unwrap();

    let orphaned = count(
        &pool,
        "SELECT count(*) FROM credit_ledger WHERE user_id = $1 AND design_id IS NULL",
        user_id,
    )
    .await;
    assert_eq!(orphaned, 1);
}

#[sqlx::test]
async fn deleting_a_user_deletes_everything_theirs(pool: PgPool) {
    let user_id = user(&pool, Some("gone@example.com")).await;
    let other = user(&pool, None).await;
    for owner in [user_id, other] {
        let design_id = design(&pool, owner).await;
        let capture_id = new_id();
        sqlx::query(
            "INSERT INTO captures (id, user_id, mode, paper, input, purge_after)
             VALUES ($1, $2, 'trace', 'letter', '{}', now() + interval '30 days')",
        )
        .bind(capture_id)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO products (id, user_id, capture_id, shape, contour_mm, bbox_mm)
             VALUES ($1, $2, $3, 'round', '[]', '{}')",
        )
        .bind(new_id())
        .bind(owner)
        .bind(capture_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO exports (id, design_id, design_version, formats, spec, paid_with) \
             VALUES ($1, $2, 1, '{svg,pdf}', '{}', 'credit')",
        )
        .bind(new_id())
        .bind(design_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO design_unlocks (id, design_id, user_id, source, edits_until)
             VALUES ($1, $2, $3, 'credit', now() + interval '30 days')",
        )
        .bind(new_id())
        .bind(design_id)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO credit_ledger (id, user_id, delta, reason, design_id) VALUES ($1, $2, -1, 'spend', $3)",
        )
        .bind(new_id())
        .bind(owner)
        .bind(design_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO subscriptions (id, user_id, paddle_subscription_id, plan, status, paddle_updated_at)
             VALUES ($1, $2, $3, 'pro_monthly', 'active', now())",
        )
        .bind(new_id())
        .bind(owner)
        .bind(format!("sub_{owner}"))
        .execute(&pool)
        .await
        .unwrap();
    }

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let tables = [
        "captures",
        "products",
        "designs",
        "design_unlocks",
        "credit_ledger",
        "subscriptions",
    ];
    for table in tables {
        // The table names are the fixed list above, never input.
        let sql = format!("SELECT count(*) FROM {table} WHERE user_id = $1");
        assert_eq!(
            count(&pool, AssertSqlSafe(sql.clone()), user_id).await,
            0,
            "{table}"
        );
        assert_eq!(
            count(&pool, AssertSqlSafe(sql), other).await,
            1,
            "{table} of the other user"
        );
    }
    let exports =
        "SELECT count(*) FROM exports e JOIN designs d ON d.id = e.design_id WHERE d.user_id = $1";
    assert_eq!(count(&pool, exports, other).await, 1);
    let all_exports: i64 = sqlx::query_scalar("SELECT count(*) FROM exports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(all_exports, 1);
}

#[sqlx::test]
async fn a_resent_paddle_event_is_stored_once(pool: PgPool) {
    let store = || {
        sqlx::query(
            "INSERT INTO paddle_events (id, event_id, event_type, occurred_at, payload)
             VALUES ($1, 'evt_01', 'transaction.completed', now(), $2)
             ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(new_id())
        .bind(json!({ "data": { "id": "txn_01" } }))
        .execute(&pool)
    };

    assert_eq!(store().await.unwrap().rows_affected(), 1);
    assert_eq!(store().await.unwrap().rows_affected(), 0);
}

#[sqlx::test]
async fn a_running_job_always_has_a_lease(pool: PgPool) {
    let result =
        sqlx::query("INSERT INTO jobs (id, kind, status) VALUES ($1, 'run_vision', 'running')")
            .bind(new_id())
            .execute(&pool)
            .await;
    assert_eq!(sql_state(result), CHECK_VIOLATION);

    let bad_kind = sqlx::query("INSERT INTO jobs (id, kind) VALUES ($1, 'Run Vision')")
        .bind(new_id())
        .execute(&pool)
        .await;
    assert_eq!(sql_state(bad_kind), CHECK_VIOLATION);
}
