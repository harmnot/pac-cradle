use std::time::Duration;

use axum::http::StatusCode;
use reqwest::{Method, Response};
use time::OffsetDateTime;

use super::{NewObject, ObjectStorage};
use crate::config::StorageConfig;
use crate::error::{AppError, ErrorCode};
use crate::shared::future::BoxFuture;
use crate::shared::sigv4::{self, Request, Signer};
use crate::third_party::http::{self, Timeouts};

/// Calls to the store are short: a photo of at most 15 MB from the VPS takes a second or two.
/// Requests that wait on it stay well inside the API's 30-second limit.
const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    total: Duration::from_secs(20),
};

/// How much of an error answer is read, for the logs.
const MAX_ERROR_BODY: usize = 2048;

/// [`ObjectStorage`] on Cloudflare R2 (or any S3-compatible store), through the S3 API with
/// path-style URLs: `{endpoint}/{bucket}/{key}`, every request signed with SigV4.
pub struct CloudflareR2 {
    client: reqwest::Client,
    /// `https://<account id>.r2.cloudflarestorage.com`.
    endpoint: String,
    /// The endpoint's host (and port, if it names one), as the `Host` header carries it.
    host: String,
    bucket: String,
    signer: Signer,
}

impl CloudflareR2 {
    /// A store with these settings. Fails only if the HTTPS client can't be set up.
    pub fn new(config: &StorageConfig) -> Result<Self, AppError> {
        let host = config
            .endpoint
            .split_once("://")
            .map_or(config.endpoint.as_str(), |(_, host)| host)
            .to_owned();
        Ok(Self {
            client: http::client(TIMEOUTS)?,
            endpoint: config.endpoint.clone(),
            host,
            bucket: config.bucket.clone(),
            signer: Signer::new((
                config.access_key_id.clone(),
                config.secret_access_key.clone(),
                config.region.clone(),
            )),
        })
    }

    fn path(&self, key: &str) -> String {
        format!("/{}/{}", self.bucket, sigv4::encode_path(key))
    }

    /// Sends a signed request for the object at `key`, with `body` (the object) for a PUT.
    async fn send(
        &self,
        (method, key, body): (Method, &str, Option<NewObject>),
    ) -> Result<Response, AppError> {
        let path = self.path(key);
        let payload_sha256 =
            sigv4::payload_hash(body.as_ref().map_or(&[], |object| object.bytes.as_slice()));
        // Sorted by name, as SigV4 signs them.
        let mut headers: Vec<(&str, &str)> = Vec::new();
        if let Some(object) = &body {
            if let Some(disposition) = &object.content_disposition {
                headers.push(("content-disposition", disposition));
            }
            headers.push(("content-type", object.content_type));
        }
        let signed = self.signer.sign((
            &Request {
                method: method.as_str(),
                host: &self.host,
                path: &path,
                headers: &headers,
                payload_sha256: &payload_sha256,
            },
            OffsetDateTime::now_utc(),
        ));
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.endpoint))
            .header("authorization", signed.authorization)
            .header("x-amz-date", signed.amz_date)
            .header("x-amz-content-sha256", signed.content_sha256);
        for &(name, value) in &headers {
            request = request.header(name, value);
        }
        if let Some(object) = body {
            request = request.body(object.bytes);
        }
        request.send().await.map_err(unavailable)
    }
}

impl ObjectStorage for CloudflareR2 {
    fn put(&self, object: NewObject) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let key = object.key.clone();
            let response = self.send((Method::PUT, &key, Some(object))).await?;
            refused(response).await
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Vec<u8>>, AppError>> {
        Box::pin(async move {
            let response = self.send((Method::GET, key, None)).await?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if !response.status().is_success() {
                return refused(response).await.map(|()| None);
            }
            let bytes = response.bytes().await.map_err(unavailable)?;
            Ok(Some(bytes.to_vec()))
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let response = self.send((Method::DELETE, key, None)).await?;
            // S3 answers 204 whether or not the object was there; some stores say 404.
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(());
            }
            refused(response).await
        })
    }

    fn signed_url(&self, (key, valid_for): (&str, Duration)) -> Result<String, AppError> {
        let path = self.path(key);
        let query = self.signer.presign((
            &Request {
                method: "GET",
                host: &self.host,
                path: &path,
                headers: &[],
                payload_sha256: sigv4::UNSIGNED_PAYLOAD,
            },
            valid_for,
            OffsetDateTime::now_utc(),
        ));
        Ok(format!("{}{path}?{query}", self.endpoint))
    }
}

/// What the store said when it refused a request.
#[derive(Debug, thiserror::Error)]
#[error("object storage answered {status} {code}")]
struct Refused {
    status: StatusCode,
    /// The S3 error code (`AccessDenied`, `NoSuchBucket`…), when the body has one.
    code: String,
}

/// `Ok` for a success; otherwise the store's refusal, read from its XML error body.
async fn refused(response: Response) -> Result<(), AppError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    let body: String = body.chars().take(MAX_ERROR_BODY).collect();
    let code = body
        .split_once("<Code>")
        .and_then(|(_, rest)| rest.split_once("</Code>"))
        .map_or_else(String::new, |(code, _)| code.to_owned());
    let error = Refused { status, code };
    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
        return Err(unavailable(error));
    }
    Err(AppError::new(ErrorCode::Internal)
        .with_meta("dependency", "storage")
        .with_source(error))
}

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::ServiceUnavailable)
        .with_meta("dependency", "storage")
        .with_source(error)
}
