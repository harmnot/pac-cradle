//! AWS Signature Version 4 for the S3 API, which Cloudflare R2 speaks (M1-D4): signed request
//! headers, and presigned URLs that work for a few minutes without credentials. HMAC and
//! SHA-256 come from `ring`; the rest is string building, so no AWS crate.

use std::time::Duration;

use ring::hmac;
use time::{OffsetDateTime, UtcOffset};

use super::crypto::sha256;
use super::secret::Secret;

const ALGORITHM: &str = "AWS4-HMAC-SHA256";
const SERVICE: &str = "s3";
/// The payload hash of a presigned URL: whoever uses it sends the body.
pub const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";
/// The longest a presigned URL may work: 7 days.
pub const MAX_PRESIGNED: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Signs requests with one access key.
pub struct Signer {
    access_key_id: String,
    secret_access_key: Secret,
    region: String,
}

/// A request to sign.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// `GET`, `PUT`, `DELETE`…
    pub method: &'a str,
    /// The `Host` header: the endpoint's host, with its port when it isn't the default.
    pub host: &'a str,
    /// The path, already encoded with [`encode_path`].
    pub path: &'a str,
    /// Other headers to sign, names in lower case: `content-type`, `range`…
    pub headers: &'a [(&'a str, &'a str)],
    /// The hex SHA-256 of the body ([`payload_hash`]), or [`UNSIGNED_PAYLOAD`].
    pub payload_sha256: &'a str,
}

/// The headers that carry a request's signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedHeaders {
    /// `Authorization`.
    pub authorization: String,
    /// `x-amz-date`.
    pub amz_date: String,
    /// `x-amz-content-sha256`: the request's payload hash.
    pub content_sha256: String,
}

impl Signer {
    /// A signer for the key `access_key_id` with its secret, for `region` (`auto` on R2).
    pub fn new((access_key_id, secret_access_key, region): (String, Secret, String)) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
        }
    }

    /// The headers to add to `request`, signed at `at`. Besides `request.headers`, the
    /// signature covers `host`, `x-amz-content-sha256` and `x-amz-date`.
    pub fn sign(&self, (request, at): (&Request<'_>, OffsetDateTime)) -> SignedHeaders {
        let (date, amz_date) = timestamps(at);
        let mut headers: Vec<(&str, &str)> = vec![
            ("host", request.host),
            ("x-amz-content-sha256", request.payload_sha256),
            ("x-amz-date", &amz_date),
        ];
        headers.extend(request.headers);
        headers.sort_unstable_by_key(|&(name, _)| name);
        let signed_names = signed_names(&headers);
        let canonical = canonical_request(Canonical {
            request,
            query: "",
            headers: &headers,
            signed_names: &signed_names,
        });
        let scope = self.scope(&date);
        let signature = self.signature(&date, &string_to_sign((&amz_date, &scope, &canonical)));
        SignedHeaders {
            authorization: format!(
                "{ALGORITHM} Credential={}/{scope}, SignedHeaders={signed_names}, Signature={signature}",
                self.access_key_id
            ),
            amz_date,
            content_sha256: request.payload_sha256.to_owned(),
        }
    }

    /// The query string (without `?`) that lets anyone make `request` for `valid_for` after
    /// `at`, at most [`MAX_PRESIGNED`]. It signs only the `host` header and leaves the payload
    /// unsigned; `request.headers` and `request.payload_sha256` are ignored.
    pub fn presign(
        &self,
        (request, valid_for, at): (&Request<'_>, Duration, OffsetDateTime),
    ) -> String {
        let (date, amz_date) = timestamps(at);
        let scope = self.scope(&date);
        let credential = format!("{}/{scope}", self.access_key_id);
        let expires = valid_for.min(MAX_PRESIGNED).as_secs().max(1).to_string();
        // Already in the order SigV4 sorts them.
        let query = [
            ("X-Amz-Algorithm", ALGORITHM),
            ("X-Amz-Credential", &credential),
            ("X-Amz-Date", &amz_date),
            ("X-Amz-Expires", &expires),
            ("X-Amz-SignedHeaders", "host"),
        ]
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name, false), encode(value, false)))
        .collect::<Vec<_>>()
        .join("&");
        let headers = [("host", request.host)];
        let canonical = canonical_request(Canonical {
            request: &Request {
                payload_sha256: UNSIGNED_PAYLOAD,
                ..*request
            },
            query: &query,
            headers: &headers,
            signed_names: "host",
        });
        let signature = self.signature(&date, &string_to_sign((&amz_date, &scope, &canonical)));
        format!("{query}&X-Amz-Signature={signature}")
    }

    fn scope(&self, date: &str) -> String {
        format!("{date}/{}/{SERVICE}/aws4_request", self.region)
    }

    /// The hex HMAC of `string_to_sign` under the key derived for `date`.
    fn signature(&self, date: &str, string_to_sign: &str) -> String {
        let secret = format!("AWS4{}", self.secret_access_key.expose());
        let key = [date, &self.region, SERVICE, "aws4_request"]
            .iter()
            .fold(secret.into_bytes(), |key, part| {
                hmac_sha256(&key, part.as_bytes())
            });
        hex(&hmac_sha256(&key, string_to_sign.as_bytes()))
    }
}

/// The hex SHA-256 of a body, for [`Request::payload_sha256`].
pub fn payload_hash(body: &[u8]) -> String {
    hex(&sha256(body))
}

/// An object key as a URL path: every byte but `A–Z a–z 0–9 - _ . ~` and `/` percent-encoded.
pub fn encode_path(key: &str) -> String {
    encode(key, true)
}

/// The parts of a canonical request.
struct Canonical<'a> {
    request: &'a Request<'a>,
    query: &'a str,
    /// Sorted by name.
    headers: &'a [(&'a str, &'a str)],
    signed_names: &'a str,
}

fn canonical_request(parts: Canonical<'_>) -> String {
    let headers: String = parts
        .headers
        .iter()
        .map(|(name, value)| format!("{name}:{}\n", value.trim()))
        .collect();
    format!(
        "{}\n{}\n{}\n{headers}\n{}\n{}",
        parts.request.method,
        parts.request.path,
        parts.query,
        parts.signed_names,
        parts.request.payload_sha256
    )
}

fn string_to_sign((amz_date, scope, canonical): (&str, &str, &str)) -> String {
    format!(
        "{ALGORITHM}\n{amz_date}\n{scope}\n{}",
        hex(&sha256(canonical.as_bytes()))
    )
}

fn signed_names(sorted: &[(&str, &str)]) -> String {
    sorted
        .iter()
        .map(|&(name, _)| name)
        .collect::<Vec<_>>()
        .join(";")
}

/// `20130524` and `20130524T000000Z` for `at`, in UTC.
fn timestamps(at: OffsetDateTime) -> (String, String) {
    let at = at.to_offset(UtcOffset::UTC);
    let date = format!("{:04}{:02}{:02}", at.year(), u8::from(at.month()), at.day());
    let amz_date = format!(
        "{date}T{:02}{:02}{:02}Z",
        at.hour(),
        at.minute(),
        at.second()
    );
    (date, amz_date)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), message)
        .as_ref()
        .to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// RFC 3986 percent-encoding as SigV4 wants it: unreserved bytes stay, `/` stays when
/// `keep_slash`, everything else becomes `%XX` in upper case.
fn encode(text: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if unreserved || (keep_slash && byte == b'/') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
