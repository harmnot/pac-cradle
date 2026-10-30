//! Designs (M1-D6), end to end on a real database: a guest saves the editor's design and
//! reads it back exactly; changes are saved from the version they were made from (another
//! tab's save is a conflict, an unchanged save keeps the version); `insert_core`'s rules
//! refuse broken designs; designs are private; "My designs" pages, filters and sorts.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::designs::{card_spec, cradle_spec, grid_spec};
use common::{Reply, TestApi, WEB_ORIGIN, auth_api, call};
use serde_json::{Value, json};
use sqlx::PgPool;

const DESIGNS: &str = "/api/client/v1/designs";

/// The spec as `insert_core` writes it back: defaults filled in, nothing else changed.
fn canonical(spec: &Value) -> Value {
    let design: insert_core::Design = serde_json::from_value(spec.clone()).unwrap();
    serde_json::to_value(design).unwrap()
}

/// A new guest's access token.
async fn guest(api: &TestApi) -> String {
    let reply = api
        .send(call(Method::POST, "/api/client/v1/auth/guest").origin(WEB_ORIGIN))
        .await;
    reply.access_token()
}

async fn create(api: &TestApi, (token, body): (&str, Value)) -> Reply {
    api.send(call(Method::POST, DESIGNS).bearer(token).json(body))
        .await
}

/// Saves `spec` as `name` and returns the design's id.
async fn saved(api: &TestApi, (token, name, spec): (&str, &str, Value)) -> String {
    let reply = create(api, (token, json!({ "name": name, "spec": spec }))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    reply.json()["design_id"].as_str().unwrap().to_owned()
}

async fn patch(api: &TestApi, (token, id, body): (&str, &str, Value)) -> Reply {
    api.send(
        call(Method::PATCH, &format!("{DESIGNS}/{id}"))
            .bearer(token)
            .json(body),
    )
    .await
}

async fn list(api: &TestApi, (token, query): (&str, &str)) -> Reply {
    api.send(call(Method::GET, &format!("{DESIGNS}?{query}")).bearer(token))
        .await
}

fn names(reply: &Reply) -> Vec<String> {
    reply.json()["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test]
async fn a_guest_saves_a_design_and_reads_it_back_exactly(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;

    let reply = create(
        &api,
        (
            &token,
            json!({ "name": "  Perfume set\n ", "spec": cradle_spec() }),
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    assert_eq!(reply.header("api-version"), "1.0.0-beta.7");
    let body = reply.json();
    assert_eq!(body["name"], "Perfume set");
    assert_eq!(body["version"], 1);
    assert_eq!(body["spec"], canonical(&cradle_spec()));
    let id = body["design_id"].as_str().unwrap();

    let read = api
        .send(call(Method::GET, &format!("{DESIGNS}/{id}")).bearer(&token))
        .await;
    assert_eq!(read.status, StatusCode::OK);
    assert_eq!(read.json(), body);
    // Stored to the last bit (float_roundtrip): the outline is the editor's copy.
    let stored = sqlx::query_scalar!("SELECT spec FROM designs")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(stored, canonical(&cradle_spec()));

    // A name may be left out.
    let reply = create(&api, (&token, json!({ "spec": card_spec() }))).await;
    assert_eq!(reply.json()["name"], "");
}

#[sqlx::test]
async fn a_change_is_saved_from_the_version_it_was_made_from(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = saved(&api, (&token, "Perfume set", cradle_spec())).await;

    let reply = patch(
        &api,
        (&token, &id, json!({ "version": 1, "name": "Perfume duo" })),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    assert_eq!(
        (
            reply.json()["version"].clone(),
            reply.json()["name"].clone()
        ),
        (json!(2), json!("Perfume duo"))
    );

    // Another tab still at version 1.
    let reply = patch(
        &api,
        (&token, &id, json!({ "version": 1, "name": "Old tab" })),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.error_code(), "DESIGN_VERSION_CONFLICT");
    assert_eq!(reply.json()["error"]["meta"]["current_version"], 2);

    // Saving the same state again keeps the version.
    let reply = patch(
        &api,
        (
            &token,
            &id,
            json!({ "version": 2, "name": "Perfume duo", "spec": cradle_spec() }),
        ),
    )
    .await;
    assert_eq!(reply.json()["version"], 2);

    // A new spec, whole.
    let mut spec = cradle_spec();
    spec["products"][0]["quantity"] = json!(3);
    let reply = patch(&api, (&token, &id, json!({ "version": 2, "spec": spec }))).await;
    assert_eq!(reply.json()["version"], 3);
    assert_eq!(reply.json()["spec"]["products"][0]["quantity"], 3);
    assert_eq!(reply.json()["name"], "Perfume duo");

    let reply = patch(&api, (&token, &id, json!({ "version": 3 }))).await;
    assert_eq!(reply.error_code(), "INVALID_REQUEST");
}

#[sqlx::test]
async fn designs_breaking_the_rules_are_refused(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;

    let mut empty = cradle_spec();
    empty["products"] = json!([]);
    let reply = create(&api, (&token, json!({ "spec": empty }))).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.error_code(), "DESIGN_INVALID");
    assert_eq!(
        reply.json()["error"]["message"],
        "This design can't be saved: a design needs at least one product."
    );

    let mut shapeless = cradle_spec();
    shapeless.as_object_mut().unwrap().remove("material");
    let reply = create(&api, (&token, json!({ "spec": shapeless }))).await;
    assert_eq!(reply.error_code(), "DESIGN_INVALID");
    assert!(
        reply.json()["error"]["meta"]["detail"]
            .as_str()
            .unwrap()
            .contains("missing field `material`")
    );

    let reply = create(
        &api,
        (
            &token,
            json!({ "name": "x".repeat(201), "spec": cradle_spec() }),
        ),
    )
    .await;
    assert_eq!(reply.error_code(), "INVALID_REQUEST");
    assert_eq!(reply.json()["error"]["meta"]["field"], "name");

    // An unknown field is a mistake, not something to drop quietly.
    let reply = create(&api, (&token, json!({ "spec": cradle_spec(), "box": {} }))).await;
    assert_eq!(reply.error_code(), "INVALID_REQUEST");

    // A broken change leaves the design as it was.
    let id = saved(&api, (&token, "Set", cradle_spec())).await;
    let mut too_many = cradle_spec();
    too_many["products"][0]["quantity"] = json!(1);
    let reply = patch(
        &api,
        (&token, &id, json!({ "version": 1, "spec": too_many })),
    )
    .await;
    assert_eq!(reply.error_code(), "DESIGN_INVALID");
    let version = sqlx::query_scalar!("SELECT version FROM designs WHERE name = 'Set'")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(version, 1);
}

#[sqlx::test]
async fn designs_are_private(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let owner = guest(&api).await;
    let other = guest(&api).await;
    let id = saved(&api, (&owner, "Mine", cradle_spec())).await;

    let reply = api
        .send(call(Method::GET, &format!("{DESIGNS}/{id}")).bearer(&other))
        .await;
    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::NOT_FOUND, "DESIGN_NOT_FOUND")
    );
    let reply = patch(
        &api,
        (&other, &id, json!({ "version": 1, "name": "Theirs" })),
    )
    .await;
    assert_eq!(reply.error_code(), "DESIGN_NOT_FOUND");
    assert_eq!(list(&api, (&other, "")).await.json()["total"], 0);
    let reply = api.send(call(Method::GET, DESIGNS)).await;
    assert_eq!(reply.error_code(), "AUTH_REQUIRED");
}

#[sqlx::test]
async fn my_designs_pages_filters_and_sorts(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    for (name, spec) in [
        ("Perfume cradle", cradle_spec()),
        ("Lipstick card", card_spec()),
        ("Candle grid", grid_spec()),
        ("perfume minis", card_spec()),
        ("Tea grid", grid_spec()),
    ] {
        saved(&api, (&token, name, spec)).await;
    }

    // Last changed first, by default.
    let reply = list(&api, (&token, "")).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        names(&reply),
        [
            "Tea grid",
            "perfume minis",
            "Candle grid",
            "Lipstick card",
            "Perfume cradle"
        ]
    );
    let body = reply.json();
    assert_eq!(
        (
            &body["page"],
            &body["page_size"],
            &body["total"],
            &body["total_pages"]
        ),
        (&json!(1), &json!(20), &json!(5), &json!(1))
    );
    let cradle = &body["items"][4];
    assert_eq!(cradle["insert_style"], "cradle_platform");
    assert_eq!(
        (
            &cradle["product_types"],
            &cradle["product_count"],
            &cradle["has_box"]
        ),
        (&json!(1), &json!(2), &json!(true))
    );
    assert!(cradle.get("spec").is_none(), "no outlines in the list");

    assert_eq!(
        names(&list(&api, (&token, "search=PERFUME")).await),
        ["perfume minis", "Perfume cradle"]
    );
    assert_eq!(
        names(&list(&api, (&token, "style=partition_grid&sort=name")).await),
        ["Candle grid", "Tea grid"]
    );
    assert_eq!(
        names(&list(&api, (&token, "sort=name")).await),
        [
            "Candle grid",
            "Lipstick card",
            "Perfume cradle",
            "perfume minis",
            "Tea grid"
        ]
    );
    assert_eq!(
        names(
            &list(
                &api,
                (&token, "sort=created_at&order=asc&page=2&page_size=2")
            )
            .await
        ),
        ["Candle grid", "perfume minis"]
    );
    let beyond = list(&api, (&token, "page=4&page_size=2")).await.json();
    assert_eq!(
        (&beyond["items"], &beyond["total"], &beyond["total_pages"]),
        (&json!([]), &json!(5), &json!(3))
    );

    for (query, field) in [
        ("page_size=0", "page_size"),
        ("page_size=101", "page_size"),
        ("page=0", "page"),
    ] {
        let reply = list(&api, (&token, query)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(reply.json()["error"]["meta"]["field"], field);
    }
    for query in ["style=round_tray", "sort=price", "colour=red"] {
        assert_eq!(
            list(&api, (&token, query)).await.error_code(),
            "INVALID_REQUEST",
            "{query}"
        );
    }
}
