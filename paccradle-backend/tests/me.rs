//! `/me` with billing (M1-D8): a guest is on the free plan with nothing; an account's plan,
//! credits, subscription and unlocked designs follow its ledger, subscriptions and unlocks;
//! each design says whether it is unlocked and whether that covers it as it is now.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::designs::{card_spec, cradle_spec};
use common::{TestApi, WEB_ORIGIN, account, add_credits, auth_api, call};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CLIENT: &str = "/api/client/v1";

async fn me(api: &TestApi, token: &str) -> Value {
    let reply = api
        .send(call(Method::GET, &format!("{CLIENT}/me")).bearer(token))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    assert_eq!(reply.header("api-version"), "1.0.0-beta.7");
    reply.json()
}

async fn subscribe(pool: &PgPool, (user, plan, status, age_days): (Uuid, &str, &str, i32)) {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions
            (id, user_id, paddle_subscription_id, plan, status, current_period_end, paddle_updated_at)
        VALUES (gen_random_uuid(), $1, 'sub_' || gen_random_uuid(), $2, $3,
                now() + interval '30 days', now() - make_interval(days => $4))
        "#,
        user,
        plan,
        status,
        age_days,
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Saves `spec` for `token` and returns the design's id.
async fn design(api: &TestApi, (token, spec): (&str, Value)) -> String {
    let reply = api
        .send(
            call(Method::POST, &format!("{CLIENT}/designs"))
                .bearer(token)
                .json(json!({ "name": "Set", "spec": spec })),
        )
        .await;
    assert_eq!(
        reply.json()["unlock"],
        Value::Null,
        "a new design isn't unlocked"
    );
    reply.json()["design_id"].as_str().unwrap().to_owned()
}

async fn get_design(api: &TestApi, (token, id): (&str, &str)) -> Value {
    api.send(call(Method::GET, &format!("{CLIENT}/designs/{id}")).bearer(token))
        .await
        .json()
}

#[sqlx::test]
async fn a_guest_is_on_the_free_plan_with_nothing(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = api
        .send(call(Method::POST, &format!("{CLIENT}/auth/guest")).origin(WEB_ORIGIN))
        .await
        .access_token();

    let body = me(&api, &token).await;

    assert_eq!(
        body["role"], "guest",
        "the account's fields stay at the top"
    );
    assert_eq!(
        body["billing"],
        json!({ "plan": "free", "credits": 0, "subscription": null, "unlocked_designs": 0 })
    );
}

#[sqlx::test]
async fn me_follows_credits_subscriptions_and_unlocks(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "brand@example.com").await;
    add_credits(&api.pool, (user.id, 5)).await;
    let id = design(&api, (&user.token, cradle_spec())).await;
    let reply = api
        .send(
            call(Method::POST, &format!("{CLIENT}/designs/{id}/exports"))
                .bearer(&user.token)
                .json(json!({ "formats": ["svg"] })),
        )
        .await;
    assert_eq!(reply.json()["paid_with"], "credit");

    let billing = &me(&api, &user.token).await["billing"];
    assert_eq!(billing["email"], Value::Null, "billing holds only billing");
    assert_eq!(
        (
            &billing["plan"],
            &billing["credits"],
            &billing["unlocked_designs"]
        ),
        (&json!("free"), &json!(4), &json!(1))
    );

    // An old canceled subscription, then a live yearly one: the live one counts.
    subscribe(&api.pool, (user.id, "pro_monthly", "canceled", 40)).await;
    assert_eq!(
        me(&api, &user.token).await["billing"]["subscription"]["status"],
        "canceled"
    );
    assert_eq!(me(&api, &user.token).await["billing"]["plan"], "free");
    subscribe(&api.pool, (user.id, "pro_yearly", "past_due", 1)).await;
    let billing = &me(&api, &user.token).await["billing"];
    assert_eq!(billing["plan"], "pro", "past due still gives the plan");
    assert_eq!(billing["subscription"]["plan"], "pro_yearly");
    assert_eq!(billing["subscription"]["status"], "past_due");
    assert!(billing["subscription"]["current_period_end"].is_string());
}

#[sqlx::test]
async fn a_design_says_whether_it_is_unlocked_and_whether_that_covers_it(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = account(&api, "maker@example.com").await;
    add_credits(&api.pool, (user.id, 1)).await;
    let unlocked = design(&api, (&user.token, cradle_spec())).await;
    let other = design(&api, (&user.token, card_spec())).await;
    api.send(
        call(
            Method::POST,
            &format!("{CLIENT}/designs/{unlocked}/exports"),
        )
        .bearer(&user.token)
        .json(json!({ "formats": ["dxf"] })),
    )
    .await;

    let body = get_design(&api, (&user.token, &unlocked)).await;
    assert_eq!(body["unlock"]["source"], "credit");
    assert_eq!(body["unlock"]["covers_current_version"], true);
    assert!(body["unlock"]["edits_until"].is_string());
    assert_eq!(
        get_design(&api, (&user.token, &other)).await["unlock"],
        Value::Null
    );
    let list = api
        .send(
            call(
                Method::GET,
                &format!("{CLIENT}/designs?sort=created_at&order=asc"),
            )
            .bearer(&user.token),
        )
        .await
        .json();
    assert_eq!(list["items"][0]["unlock"]["source"], "credit");
    assert_eq!(list["items"][1]["unlock"], Value::Null);

    // Past its window, a change made now isn't covered: the next export pays again.
    sqlx::query!("UPDATE design_unlocks SET edits_until = now() - interval '1 day'")
        .execute(&api.pool)
        .await
        .unwrap();
    let reply = api
        .send(
            call(Method::PATCH, &format!("{CLIENT}/designs/{unlocked}"))
                .bearer(&user.token)
                .json(json!({ "version": 1, "name": "Set two" })),
        )
        .await;
    assert_eq!(reply.json()["unlock"]["covers_current_version"], false);
    let reply = api
        .send(
            call(
                Method::POST,
                &format!("{CLIENT}/designs/{unlocked}/exports"),
            )
            .bearer(&user.token)
            .json(json!({ "formats": ["dxf"] })),
        )
        .await;
    assert_eq!(
        reply.error_code(),
        "EXPORT_PAYMENT_REQUIRED",
        "no credit left"
    );
}
