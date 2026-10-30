//! Exports (M1-D7), end to end on a real database with files kept in memory: guests must sign
//! up first; the PNG preview and test print are free; production files are covered by an
//! unlock still in its edit window, then Pro, then a credit, else a 402 with the checkout;
//! nothing is charged for a design that can't be made; a job makes the files and the export
//! gives 5-minute links; history pages; a failed export can be asked for again for free.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::designs::{cradle_spec, grid_spec};
use common::{Reply, TestApi, WEB_ORIGIN, auth_api, call, link_token};
use paccradle_backend::jobs;
use paccradle_backend::models::export::RunExport;
use paccradle_backend::models::job::{JobKind, JobOutcome};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CLIENT: &str = "/api/client/v1";

/// Checkout prices, as F1 will configure them.
const PRICES: [(&str, &str); 4] = [
    ("PADDLE_PRICE_SINGLE_EXPORT", "pri_01single"),
    ("PADDLE_PRICE_EXPORT_PACK", "pri_01pack"),
    ("PADDLE_PRICE_PRO_MONTHLY", "pri_01monthly"),
    ("PADDLE_PRICE_PRO_YEARLY", "pri_01yearly"),
];

/// A signed-in account.
struct Account {
    token: String,
    id: Uuid,
}

/// A new account for `email`, signed up through its confirmation link.
async fn account(api: &TestApi, email: &str) -> Account {
    let reply = api
        .send(
            call(Method::POST, &format!("{CLIENT}/auth/email-link"))
                .json(json!({ "email": email })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    let sent = api.deliver_emails().await;
    let reply = api
        .send(
            call(Method::POST, &format!("{CLIENT}/auth/email-link/sign-in"))
                .json(json!({ "token": link_token(&sent[0]) })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    Account {
        token: reply.access_token(),
        id: reply.json()["account"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    }
}

/// Saves `spec` as "Perfume set" for `token` and returns the design's id.
async fn design(api: &TestApi, (token, spec): (&str, Value)) -> String {
    let reply = api
        .send(
            call(Method::POST, &format!("{CLIENT}/designs"))
                .bearer(token)
                .json(json!({ "name": "Perfume set!", "spec": spec })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    reply.json()["design_id"].as_str().unwrap().to_owned()
}

async fn export(api: &TestApi, (token, design, body): (&str, &str, Value)) -> Reply {
    api.send(
        call(Method::POST, &format!("{CLIENT}/designs/{design}/exports"))
            .bearer(token)
            .json(body),
    )
    .await
}

async fn get_export(api: &TestApi, (token, id): (&str, &str)) -> Reply {
    api.send(call(Method::GET, &format!("{CLIENT}/exports/{id}")).bearer(token))
        .await
}

/// Runs every queued export job, as the worker would.
async fn run_export_jobs(api: &TestApi) -> Vec<JobOutcome> {
    let handlers = jobs::handlers(&api.state);
    let mut outcomes = Vec::new();
    while let Some(job) = api.state.jobs.claim(&[RunExport::NAME]).await.unwrap() {
        let result = handlers.run(&job).await;
        outcomes.push(api.state.jobs.finish(&job, result).await.unwrap());
    }
    outcomes
}

/// Gives `user` `count` export credits, as a pack purchase would.
async fn add_credits(pool: &PgPool, (user, count): (Uuid, i32)) {
    sqlx::query!(
        r#"
        INSERT INTO credit_ledger (id, user_id, delta, reason, paddle_transaction_id)
        VALUES (gen_random_uuid(), $1, $2, 'purchase', 'txn_' || gen_random_uuid())
        "#,
        user,
        count,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn balance(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT coalesce(sum(delta), 0)::bigint AS "balance!" FROM credit_ledger WHERE user_id = $1"#,
        user
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn export_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM exports"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Moves the design's unlock window into the past and saves a change after it.
async fn outlive_edit_window(api: &TestApi, (token, design): (&str, &str)) {
    sqlx::query!("UPDATE design_unlocks SET edits_until = now() - interval '1 day'")
        .execute(&api.pool)
        .await
        .unwrap();
    let version = api
        .send(call(Method::GET, &format!("{CLIENT}/designs/{design}")).bearer(token))
        .await
        .json()["version"]
        .clone();
    let reply = api
        .send(
            call(Method::PATCH, &format!("{CLIENT}/designs/{design}"))
                .bearer(token)
                .json(json!({ "version": version, "name": format!("Perfume set {version}") })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
}

fn paid_with(reply: &Reply) -> String {
    assert_eq!(reply.status, StatusCode::ACCEPTED, "{:?}", reply.body);
    reply.json()["paid_with"].as_str().unwrap().to_owned()
}

#[sqlx::test]
async fn guests_sign_up_before_any_export(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let guest = api
        .send(call(Method::POST, &format!("{CLIENT}/auth/guest")).origin(WEB_ORIGIN))
        .await
        .access_token();
    let id = design(&api, (&guest, cradle_spec())).await;

    let reply = export(&api, (&guest, &id, json!({ "formats": ["png"] }))).await;

    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::FORBIDDEN, "AUTH_ACCOUNT_REQUIRED")
    );
    assert_eq!(export_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn the_preview_and_the_test_print_are_free(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    let id = design(&api, (&user.token, cradle_spec())).await;

    let reply = export(
        &api,
        (
            &user.token,
            &id,
            json!({ "formats": ["png", "test_print", "png"], "paper": "letter" }),
        ),
    )
    .await;

    assert_eq!(paid_with(&reply), "free");
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.header("api-version"), "1.0.0-beta.6");
    let body = reply.json();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["formats"], json!(["png", "test_print"]));
    assert_eq!(body["files"], json!([]));
    let export_id = body["export_id"].as_str().unwrap().to_owned();

    assert_eq!(run_export_jobs(&api).await, [JobOutcome::Done]);

    let body = get_export(&api, (&user.token, &export_id)).await.json();
    assert_eq!(body["status"], "done", "{body}");
    let files = body["files"].as_array().unwrap();
    let names: Vec<&str> = files
        .iter()
        .map(|f| f["file_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "paccradle-perfume-set-v1-preview.png",
            "paccradle-perfume-set-v1-test-print-letter.pdf"
        ]
    );
    for (file, magic) in files.iter().zip([b"\x89PNG".as_slice(), b"%PDF"]) {
        let key = format!(
            "exports/{export_id}/{}",
            file["file_name"].as_str().unwrap()
        );
        assert!(
            file["url"]
                .as_str()
                .unwrap()
                .starts_with(&format!("memory:///{key}?"))
        );
        let (bytes, _) = api.storage.object(&key).unwrap();
        assert!(bytes.starts_with(magic));
        assert_eq!(file["bytes"], bytes.len());
        assert_eq!(
            api.storage.content_disposition(&key).unwrap(),
            format!(
                "attachment; filename=\"{}\"",
                file["file_name"].as_str().unwrap()
            )
        );
    }
    assert_eq!(balance(&api.pool, user.id).await, 0);
}

#[sqlx::test]
async fn production_files_need_the_design_paid_for(pool: PgPool) {
    let api = auth_api(pool, &PRICES);
    let user = account(&api, "brand@example.com").await;
    let id = design(&api, (&user.token, cradle_spec())).await;
    let ask = || json!({ "formats": ["svg", "dxf", "pdf"] });

    // Nothing pays for it: the checkout for a single export of this design.
    let reply = export(&api, (&user.token, &id, ask())).await;
    assert_eq!(reply.status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(reply.error_code(), "EXPORT_PAYMENT_REQUIRED");
    let meta = &reply.json()["error"]["meta"];
    assert_eq!(
        meta["checkout"],
        json!({
            "price_id": "pri_01single",
            "custom_data": { "user_id": user.id, "design_id": id },
            "customer_email": "brand@example.com"
        })
    );
    assert_eq!(meta["offers"]["export_pack"], "pri_01pack");
    assert_eq!(export_count(&api.pool).await, 0);

    // A credit: spent once, and the design is unlocked for 30 days.
    add_credits(&api.pool, (user.id, 2)).await;
    assert_eq!(
        paid_with(&export(&api, (&user.token, &id, ask())).await),
        "credit"
    );
    assert_eq!(balance(&api.pool, user.id).await, 1);
    let unlock = sqlx::query!(
        r#"SELECT source, extract(epoch FROM edits_until - unlocked_at)::float8 AS "days!" FROM design_unlocks"#
    )
    .fetch_one(&api.pool)
    .await
    .unwrap();
    assert_eq!(unlock.source, "credit");
    assert!((unlock.days / 86_400.0 - 30.0).abs() < 1e-6);

    // Asked again, or changed and asked again within the window: covered.
    assert_eq!(
        paid_with(&export(&api, (&user.token, &id, ask())).await),
        "unlocked"
    );
    let reply = api
        .send(
            call(Method::PATCH, &format!("{CLIENT}/designs/{id}"))
                .bearer(&user.token)
                .json(json!({ "version": 1, "name": "Perfume duo" })),
        )
        .await;
    assert_eq!(reply.json()["version"], 2);
    assert_eq!(
        paid_with(&export(&api, (&user.token, &id, ask())).await),
        "unlocked"
    );
    assert_eq!(balance(&api.pool, user.id).await, 1);

    // A version saved after the window: the next credit.
    outlive_edit_window(&api, (&user.token, &id)).await;
    assert_eq!(
        paid_with(&export(&api, (&user.token, &id, ask())).await),
        "credit"
    );
    assert_eq!(balance(&api.pool, user.id).await, 0);
    outlive_edit_window(&api, (&user.token, &id)).await;
    let reply = export(&api, (&user.token, &id, ask())).await;
    assert_eq!(reply.error_code(), "EXPORT_PAYMENT_REQUIRED");

    // The files: the production writers' own formats.
    assert_eq!(run_export_jobs(&api).await.len(), 4);
    let first = sqlx::query_scalar!("SELECT id FROM exports ORDER BY created_at LIMIT 1")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    let body = get_export(&api, (&user.token, &first.to_string()))
        .await
        .json();
    let names: Vec<&str> = body["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["file_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "paccradle-perfume-set-v1.svg",
            "paccradle-perfume-set-v1.dxf",
            "paccradle-perfume-set-v1.pdf"
        ]
    );
    let (svg, content_type) = api
        .storage
        .object(&format!("exports/{first}/paccradle-perfume-set-v1.svg"))
        .unwrap();
    assert_eq!(content_type, "image/svg+xml");
    assert!(svg.starts_with(b"<?xml"));
}

#[sqlx::test]
async fn pro_unlocks_without_spending_credits(pool: PgPool) {
    let api = auth_api(pool, &PRICES);
    let user = account(&api, "studio@example.com").await;
    let id = design(&api, (&user.token, cradle_spec())).await;
    add_credits(&api.pool, (user.id, 1)).await;
    let subscribe = |status: &'static str| {
        let pool = api.pool.clone();
        let user = user.id;
        async move {
            sqlx::query!(
                r#"
                INSERT INTO subscriptions (id, user_id, paddle_subscription_id, plan, status, paddle_updated_at)
                VALUES (gen_random_uuid(), $1, 'sub_' || gen_random_uuid(), 'pro_monthly', $2, now())
                "#,
                user,
                status,
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    };
    subscribe("active").await;

    let reply = export(&api, (&user.token, &id, json!({ "formats": ["svg"] }))).await;

    assert_eq!(paid_with(&reply), "pro");
    assert_eq!(balance(&api.pool, user.id).await, 1, "the credit is kept");
    let source = sqlx::query_scalar!("SELECT source FROM design_unlocks")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(source, "pro");

    // A canceled subscription no longer covers a new version past the window.
    sqlx::query!("UPDATE subscriptions SET status = 'canceled'")
        .execute(&api.pool)
        .await
        .unwrap();
    outlive_edit_window(&api, (&user.token, &id)).await;
    let reply = export(&api, (&user.token, &id, json!({ "formats": ["svg"] }))).await;
    assert_eq!(paid_with(&reply), "credit");
}

#[sqlx::test]
async fn a_design_that_cant_be_made_costs_nothing(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    add_credits(&api.pool, (user.id, 1)).await;
    // A 1 × 1 grid with no frame: valid, but there's nothing to cut.
    let mut spec = grid_spec();
    spec["insert"]["rows"] = json!(1);
    spec["insert"]["columns"] = json!(1);
    spec["products"][0]["quantity"] = json!(1);
    let id = design(&api, (&user.token, spec)).await;

    let reply = export(&api, (&user.token, &id, json!({ "formats": ["svg"] }))).await;

    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.error_code(), "DESIGN_NOT_BUILDABLE");
    assert_eq!(
        reply.json()["error"]["message"],
        "This design can't be made yet: a 1 × 1 grid without a frame has no dividers to cut."
    );
    assert_eq!(balance(&api.pool, user.id).await, 1);
    assert_eq!(export_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn requests_are_checked(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    let id = design(&api, (&user.token, cradle_spec())).await;

    let reply = export(
        &api,
        (
            &user.token,
            &id,
            json!({ "formats": ["png"], "version": 2 }),
        ),
    )
    .await;
    assert_eq!(reply.error_code(), "DESIGN_VERSION_CONFLICT");
    assert_eq!(reply.json()["error"]["meta"]["current_version"], 1);
    let reply = export(&api, (&user.token, &id, json!({ "formats": [] }))).await;
    assert_eq!(reply.json()["error"]["meta"]["field"], "formats");
    let reply = export(
        &api,
        (&user.token, &id, json!({ "formats": ["test_print"] })),
    )
    .await;
    assert_eq!(reply.json()["error"]["meta"]["field"], "paper");
    let reply = export(&api, (&user.token, &id, json!({ "formats": ["cf2"] }))).await;
    assert_eq!(reply.error_code(), "INVALID_REQUEST");
    let unknown = Uuid::now_v7();
    let reply = export(
        &api,
        (
            &user.token,
            &unknown.to_string(),
            json!({ "formats": ["png"] }),
        ),
    )
    .await;
    assert_eq!(reply.error_code(), "DESIGN_NOT_FOUND");
    assert_eq!(export_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn exports_are_private_and_their_history_pages(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let owner = account(&api, "owner@example.com").await;
    let other = account(&api, "other@example.com").await;
    let id = design(&api, (&owner.token, cradle_spec())).await;
    let mut ids = Vec::new();
    for _ in 0..3 {
        let reply = export(&api, (&owner.token, &id, json!({ "formats": ["png"] }))).await;
        ids.push(reply.json()["export_id"].as_str().unwrap().to_owned());
    }
    run_export_jobs(&api).await;
    export(&api, (&owner.token, &id, json!({ "formats": ["png"] }))).await;

    let reply = get_export(&api, (&other.token, &ids[0])).await;
    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND")
    );
    let history = |query: &'static str| {
        let token = owner.token.clone();
        let url = format!("{CLIENT}/designs/{id}/exports?{query}");
        let api = &api;
        async move { api.send(call(Method::GET, &url).bearer(&token)).await }
    };
    let page = history("").await.json();
    assert_eq!(
        (&page["total"], &page["total_pages"]),
        (&json!(4), &json!(1))
    );
    assert_eq!(page["items"][0]["status"], "pending", "newest first");
    let page = history("status=done&order=asc&page_size=2&page=2")
        .await
        .json();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["export_id"], ids[2].as_str());
    assert_eq!(page["total_pages"], 2);
    let reply = api
        .send(call(Method::GET, &format!("{CLIENT}/designs/{id}/exports")).bearer(&other.token))
        .await;
    assert_eq!(reply.error_code(), "DESIGN_NOT_FOUND");
    assert_eq!(history("sort=size").await.error_code(), "INVALID_REQUEST");
}

#[sqlx::test]
async fn a_failed_export_can_be_asked_for_again_for_free(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    add_credits(&api.pool, (user.id, 1)).await;
    let id = design(&api, (&user.token, cradle_spec())).await;
    let reply = export(&api, (&user.token, &id, json!({ "formats": ["svg"] }))).await;
    let export_id = reply.json()["export_id"].as_str().unwrap().to_owned();
    api.storage.set_available(false);

    assert!(matches!(
        run_export_jobs(&api).await[..],
        [JobOutcome::Retrying(_)]
    ));
    sqlx::query!("UPDATE jobs SET run_at = now(), max_attempts = attempts + 1")
        .execute(&api.pool)
        .await
        .unwrap();
    assert_eq!(run_export_jobs(&api).await, [JobOutcome::Dead]);

    let body = get_export(&api, (&user.token, &export_id)).await.json();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "EXPORT_FAILED");
    api.storage.set_available(true);
    let reply = export(&api, (&user.token, &id, json!({ "formats": ["svg"] }))).await;
    assert_eq!(paid_with(&reply), "unlocked");
    assert_eq!(balance(&api.pool, user.id).await, 0, "one credit in all");
}

#[sqlx::test]
async fn two_clicks_at_once_spend_one_credit(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    add_credits(&api.pool, (user.id, 2)).await;
    let id = design(&api, (&user.token, cradle_spec())).await;
    let ask = || json!({ "formats": ["svg"] });

    let (first, second) = tokio::join!(
        export(&api, (&user.token, &id, ask())),
        export(&api, (&user.token, &id, ask()))
    );

    let mut paid = [paid_with(&first), paid_with(&second)];
    paid.sort();
    assert_eq!(paid, ["credit", "unlocked"]);
    assert_eq!(balance(&api.pool, user.id).await, 1);
}
