//! Object storage (M1-D4): S3 request signing checked against the worked examples in AWS's
//! SigV4 documentation for S3, and the in-memory store tests and development use.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use paccradle_backend::config::{AppConfig, StorageConfig};
use paccradle_backend::shared::secret::Secret;
use paccradle_backend::shared::sigv4::{self, Request, Signer};
use paccradle_backend::third_party::http::{self, Timeouts};
use paccradle_backend::third_party::storage::{
    CloudflareR2, MemoryStorage, NewObject, ObjectStorage,
};
use time::OffsetDateTime;

/// The example key from AWS's documentation.
fn aws_example_signer() -> Signer {
    Signer::new((
        "AKIAIOSFODNN7EXAMPLE".to_owned(),
        Secret::new("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_owned()),
        "us-east-1".to_owned(),
    ))
}

const HOST: &str = "examplebucket.s3.amazonaws.com";

/// 24 May 2013, midnight UTC: when AWS's examples are signed.
fn example_time() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_369_353_600).unwrap()
}

#[test]
fn a_get_is_signed_as_in_the_aws_example() {
    let signed = aws_example_signer().sign((
        &Request {
            method: "GET",
            host: HOST,
            path: "/test.txt",
            headers: &[("range", "bytes=0-9")],
            payload_sha256: &sigv4::payload_hash(b""),
        },
        example_time(),
    ));
    assert_eq!(signed.amz_date, "20130524T000000Z");
    assert_eq!(
        signed.content_sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        signed.authorization,
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
         SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
         Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn a_put_with_an_encoded_key_is_signed_as_in_the_aws_example() {
    let body = b"Welcome to Amazon S3.";
    let signed = aws_example_signer().sign((
        &Request {
            method: "PUT",
            host: HOST,
            path: &sigv4::encode_path("/test$file.text"),
            headers: &[
                ("date", "Fri, 24 May 2013 00:00:00 GMT"),
                ("x-amz-storage-class", "REDUCED_REDUNDANCY"),
            ],
            payload_sha256: &sigv4::payload_hash(body),
        },
        example_time(),
    ));
    assert_eq!(
        signed.content_sha256,
        "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
    );
    assert_eq!(
        signed.authorization,
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
         SignedHeaders=date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class, \
         Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
    );
}

#[test]
fn a_presigned_get_matches_the_aws_example() {
    let query = aws_example_signer().presign((
        &Request {
            method: "GET",
            host: HOST,
            path: "/test.txt",
            headers: &[],
            payload_sha256: "",
        },
        Duration::from_secs(86_400),
        example_time(),
    ));
    assert_eq!(
        query,
        "X-Amz-Algorithm=AWS4-HMAC-SHA256\
         &X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request\
         &X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host\
         &X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
    );
}

#[test]
fn presigned_urls_last_at_most_a_week_and_paths_keep_only_safe_bytes() {
    let query = aws_example_signer().presign((
        &Request {
            method: "GET",
            host: HOST,
            path: "/test.txt",
            headers: &[],
            payload_sha256: "",
        },
        Duration::from_secs(30 * 24 * 60 * 60),
        example_time(),
    ));
    assert!(query.contains("&X-Amz-Expires=604800&"));
    assert_eq!(
        sigv4::encode_path("photos/0192 é+x~_-.jpg"),
        "photos/0192%20%C3%A9%2Bx~_-.jpg"
    );
}

fn r2_config() -> StorageConfig {
    StorageConfig {
        endpoint: "https://account.r2.cloudflarestorage.com".to_owned(),
        region: "auto".to_owned(),
        bucket: "paccradle-photos".to_owned(),
        access_key_id: "key-id".to_owned(),
        secret_access_key: Secret::new("key-secret".to_owned()),
    }
}

#[test]
fn r2_signed_urls_are_path_style_and_short_lived() {
    let r2 = CloudflareR2::new(&r2_config()).unwrap();

    let url = r2
        .signed_url(("photos/0192.jpg", Duration::from_secs(600)))
        .unwrap();

    let (address, query) = url.split_once('?').unwrap();
    assert_eq!(
        address,
        "https://account.r2.cloudflarestorage.com/paccradle-photos/photos/0192.jpg"
    );
    assert!(query.starts_with("X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=key-id%2F"));
    assert!(query.contains("%2Fauto%2Fs3%2Faws4_request&X-Amz-Date="));
    assert!(query.contains("&X-Amz-Expires=600&X-Amz-SignedHeaders=host&X-Amz-Signature="));
    assert!(!url.contains("key-secret"));
}

#[tokio::test]
async fn the_memory_store_keeps_deletes_and_goes_down_on_demand() {
    let store = MemoryStorage::new();
    store
        .put(NewObject {
            key: "photos/a.jpg".to_owned(),
            bytes: b"photo".to_vec(),
            content_type: "image/jpeg",
            content_disposition: None,
        })
        .await
        .unwrap();
    assert_eq!(
        store.get("photos/a.jpg").await.unwrap(),
        Some(b"photo".to_vec())
    );
    assert_eq!(store.get("photos/none.jpg").await.unwrap(), None);
    store.delete("photos/a.jpg").await.unwrap();
    store.delete("photos/a.jpg").await.unwrap();
    assert!(store.keys().is_empty());

    store.set_available(false);
    let error = store.get("photos/a.jpg").await.unwrap_err();
    assert_eq!(error.code().as_str(), "SERVICE_UNAVAILABLE");
}

/// A round trip through the bucket in `.env` (or the environment): put, read, download through
/// a signed URL, delete. Run it by hand after changing the R2 code or settings:
/// `cargo test --test storage -- --ignored`. It leaves nothing behind.
#[tokio::test]
#[ignore = "needs STORAGE_* for a real bucket and the network"]
async fn a_real_bucket_round_trip() {
    let config = AppConfig::load().unwrap();
    let r2 = CloudflareR2::new(&config.storage.expect("STORAGE_* set")).unwrap();
    let key = format!("checks/{}.txt", paccradle_backend::shared::id::new_id());
    let body = b"paccradle storage check".to_vec();

    r2.put(NewObject {
        key: key.clone(),
        bytes: body.clone(),
        content_type: "text/plain",
        content_disposition: None,
    })
    .await
    .unwrap();
    assert_eq!(r2.get(&key).await.unwrap(), Some(body.clone()));

    let client = http::client(Timeouts {
        connect: Duration::from_secs(5),
        total: Duration::from_secs(20),
    })
    .unwrap();
    let url = r2.signed_url((&key, Duration::from_secs(60))).unwrap();
    let download = client.get(&url).send().await.unwrap();
    assert_eq!(download.status().as_u16(), 200);
    assert_eq!(download.bytes().await.unwrap().to_vec(), body);
    // Without the signature, the object is private.
    let (unsigned, _) = url.split_once('?').unwrap();
    let refused = client.get(unsigned).send().await.unwrap();
    assert!(refused.status().is_client_error(), "{}", refused.status());

    r2.delete(&key).await.unwrap();
    assert_eq!(r2.get(&key).await.unwrap(), None);
    r2.delete(&key).await.unwrap();
}
