//! Sign-in building blocks (M1-D3): base64url, access-token JWTs, password hashes, tokens,
//! the CBOR reader and the new settings.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;

use paccradle_backend::config::{AppConfig, ConfigError, SmtpSecurity};
use paccradle_backend::shared::base64url;
use paccradle_backend::shared::crypto::{NewToken, token_hash};
use paccradle_backend::shared::jwt::{self, JwtError, JwtKey};
use paccradle_backend::shared::password;
use paccradle_backend::shared::webauthn::cbor::{self, Value};
use serde::{Deserialize, Serialize};

#[test]
fn base64url_round_trips_every_length_without_padding() {
    for len in 0..64 {
        let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
        let text = base64url::encode(&bytes);
        assert!(!text.contains('='), "{text}");
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_eq!(base64url::decode(&text), Some(bytes), "{len}");
    }
    // RFC 4648 test vectors, unpadded and URL-safe.
    assert_eq!(base64url::encode(b"foobar"), "Zm9vYmFy");
    assert_eq!(base64url::encode(b"fooba"), "Zm9vYmE");
    assert_eq!(base64url::encode(&[0xfb, 0xff]), "-_8");
}

#[test]
fn base64url_refuses_anything_but_the_one_canonical_text() {
    for bad in [
        "Zm9vYmE=", "Zm9v+mE", "Zm9v/mE", "Z", "Zm9vY", "Zm9 v", "Zh",
    ] {
        assert_eq!(base64url::decode(bad), None, "{bad}");
    }
    // "Zg" is "f"; "Zh" would set bits past the last byte.
    assert_eq!(base64url::decode("Zg"), Some(b"f".to_vec()));
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[test]
fn a_jwt_reads_back_only_with_its_key_and_untouched() {
    let key = JwtKey::new(&[7u8; 32]);
    let claims = Claims {
        sub: "ana".to_owned(),
        exp: 42,
    };
    let token = jwt::encode(&key, &claims).unwrap();
    assert!(token.starts_with("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9."));
    assert_eq!(jwt::decode::<Claims>(&key, &token).unwrap(), claims);

    let other_key = JwtKey::new(&[8u8; 32]);
    assert_eq!(
        jwt::decode::<Claims>(&other_key, &token),
        Err(JwtError::BadSignature)
    );

    // Another payload under the same signature.
    let parts: Vec<&str> = token.split('.').collect();
    let forged_payload = base64url::encode(br#"{"sub":"admin","exp":42}"#);
    let forged = format!("{}.{forged_payload}.{}", parts[0], parts[2]);
    assert_eq!(
        jwt::decode::<Claims>(&key, &forged),
        Err(JwtError::BadSignature)
    );

    // alg "none", with or without a signature, is never read.
    let none_header = base64url::encode(br#"{"alg":"none","typ":"JWT"}"#);
    for unsigned in [
        format!("{none_header}.{}.", parts[1]),
        format!("{none_header}.{}.{}", parts[1], parts[2]),
    ] {
        assert_eq!(
            jwt::decode::<Claims>(&key, &unsigned),
            Err(JwtError::Malformed)
        );
    }
    for junk in ["", "a.b", "a.b.c.d", &"x".repeat(5000)] {
        assert!(jwt::decode::<Claims>(&key, junk).is_err(), "{junk}");
    }
}

#[test]
fn passwords_hash_with_argon2id_and_a_fresh_salt() {
    let first = password::hash("correct horse battery").unwrap();
    let second = password::hash("correct horse battery").unwrap();

    assert!(
        first.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
        "{first}"
    );
    assert_ne!(first, second, "each hash has its own salt");
    assert!(password::verify("correct horse battery", &first));
    assert!(password::verify("correct horse battery", &second));
    assert!(!password::verify("correct horse batterY", &first));
    assert!(!password::verify("anything", "not a hash"));
}

#[test]
fn the_dummy_hash_is_a_real_argon2id_hash_that_matches_nothing() {
    let real = password::hash("x").unwrap();
    // Same algorithm and cost, so checking it takes as long as checking a real one.
    assert_eq!(
        real.rsplitn(3, '$').nth(2),
        password::DUMMY_HASH.rsplitn(3, '$').nth(2)
    );
    assert!(argon2::password_hash::phc::PasswordHash::new(password::DUMMY_HASH).is_ok());
    assert!(!password::verify("", password::DUMMY_HASH));
    assert!(!password::verify("password", password::DUMMY_HASH));
}

#[test]
fn tokens_are_random_and_stored_as_their_hash() {
    let a = NewToken::generate().unwrap();
    let b = NewToken::generate().unwrap();

    assert_eq!(a.token.len(), 43, "32 bytes in base64url");
    assert_ne!(a.token, b.token);
    assert_eq!(a.hash, token_hash(&a.token));
    assert_ne!(a.hash, token_hash(&b.token));
}

#[test]
fn cbor_reads_the_items_passkeys_use() {
    // {1: 2, 3: -7, -2: h'0102', "fmt": "none", "list": [true, null]}
    let bytes = [
        0xa5, 0x01, 0x02, 0x03, 0x26, 0x21, 0x42, 0x01, 0x02, 0x63, b'f', b'm', b't', 0x64, b'n',
        b'o', b'n', b'e', 0x64, b'l', b'i', b's', b't', 0x82, 0xf5, 0xf6,
    ];
    let value = cbor::decode(&bytes).unwrap();

    assert_eq!(value.get_int(1).and_then(Value::as_int), Some(2));
    assert_eq!(value.get_int(3).and_then(Value::as_int), Some(-7));
    assert_eq!(
        value.get_int(-2).and_then(Value::as_bytes),
        Some(&[1u8, 2][..])
    );
    assert_eq!(value.get_text("fmt").and_then(Value::as_text), Some("none"));
    assert_eq!(
        value.get_text("list"),
        Some(&Value::Array(vec![Value::Bool(true), Value::Null]))
    );
    // Wider integers: 1000 and -1000.
    assert_eq!(
        cbor::decode(&[0x19, 0x03, 0xe8]).unwrap().as_int(),
        Some(1000)
    );
    assert_eq!(
        cbor::decode(&[0x39, 0x03, 0xe7]).unwrap().as_int(),
        Some(-1000)
    );
}

#[test]
fn cbor_refuses_what_authenticators_never_send_and_input_that_lies_about_sizes() {
    let refused: [&[u8]; 9] = [
        &[],
        &[0x01, 0x02],                                           // bytes after the item
        &[0x5f, 0x41, 0x00, 0xff],                               // indefinite-length bytes
        &[0xc1, 0x01],                                           // a tag
        &[0xfb, 0, 0, 0, 0, 0, 0, 0, 0],                         // a float
        &[0x5a, 0xff, 0xff, 0xff, 0xff], // 4 GB of bytes promised, none there
        &[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], // 2^64 items promised
        &[0x63, 0xff, 0xfe, 0xfd],       // text that isn't UTF-8
        &[0x81; 20],                     // arrays nested 20 deep
    ];
    for bytes in refused {
        assert!(cbor::decode(bytes).is_err(), "{bytes:02x?}");
    }
    // Well-formed but nested past the cap.
    let nested = |depth: usize| [vec![0x81; depth], vec![0x00]].concat();
    assert!(cbor::decode(&nested(3)).is_ok());
    assert!(cbor::decode(&nested(20)).is_err());
}

fn load(vars: &[(&str, &str)]) -> Result<AppConfig, ConfigError> {
    let vars: HashMap<&str, &str> = vars.iter().copied().collect();
    AppConfig::from_lookup(|name| vars.get(name).map(|value| (*value).to_owned()))
}

const PRODUCTION: [(&str, &str); 13] = [
    ("APP_ENV", "production"),
    (
        "DATABASE_URL",
        "postgres://api:secret@db.internal:5432/paccradle",
    ),
    ("APP_WEB_URL", "https://app.paccradle.com"),
    ("ADMIN_WEB_URL", "https://admin.paccradle.com"),
    ("WEBAUTHN_RP_ID", "paccradle.com"),
    ("AUTH_JWT_SECRET", "0123456789abcdef0123456789abcdef"),
    ("SMTP_HOST", "mail.smtp2go.com"),
    ("SMTP_USERNAME", "paccradle"),
    ("SMTP_PASSWORD", "smtp-secret"),
    // Photos (M1-D4): tests/config.rs checks these.
    (
        "STORAGE_ENDPOINT",
        "https://account.r2.cloudflarestorage.com",
    ),
    ("STORAGE_BUCKET", "paccradle-photos"),
    ("STORAGE_ACCESS_KEY_ID", "key-id"),
    ("STORAGE_SECRET_ACCESS_KEY", "key-secret"),
];

#[test]
fn development_signs_in_with_no_settings() {
    let config = common::config(&[]);

    assert_eq!(config.auth.rp_id, "localhost");
    assert_eq!(config.auth.rp_name, "Paccradle");
    assert_eq!(config.auth.jwt_secret.expose().len(), 32, "a random one");
    assert_eq!(config.rate_limit_auth_per_minute.get(), 20);
    // Mailpit.
    assert_eq!(config.email.smtp_host, "127.0.0.1");
    assert_eq!(config.email.smtp_port, 1025);
    assert_eq!(config.email.smtp_security, SmtpSecurity::None);
    assert!(config.email.smtp_credentials.is_none());
    // Each start makes its own secret.
    assert_ne!(
        common::config(&[]).auth.jwt_secret.expose(),
        config.auth.jwt_secret.expose()
    );
}

#[test]
fn production_needs_sign_in_settings_and_a_shared_passkey_domain() {
    let full = [
        &PRODUCTION[..],
        &[("EMAIL_FROM", "Paccradle <no-reply@paccradle.com>")],
    ]
    .concat();
    let config = load(&full).unwrap();
    assert_eq!(config.auth.rp_id, "paccradle.com");
    assert_eq!(config.email.smtp_security, SmtpSecurity::StartTls);
    assert_eq!(config.email.smtp_port, 587);
    // Secrets never show in debug output.
    assert!(!format!("{config:?}").contains("smtp-secret"));
    assert!(!format!("{config:?}").contains("0123456789abcdef"));

    let without = |name: &str| -> Vec<(&str, &str)> {
        full.iter().copied().filter(|(n, _)| *n != name).collect()
    };
    for name in [
        "AUTH_JWT_SECRET",
        "SMTP_HOST",
        "EMAIL_FROM",
        "SMTP_PASSWORD",
    ] {
        let error = load(&without(name)).unwrap_err().to_string();
        assert_eq!(error, format!("{name} is required in production"));
    }
    let error = |vars: &[(&str, &str)]| load(&[&full[..], vars].concat()).unwrap_err().to_string();
    assert!(error(&[("AUTH_JWT_SECRET", "short")]).starts_with("AUTH_JWT_SECRET"));
    assert_eq!(
        error(&[("AUTH_JWT_SECRET", "CHANGE_ME_AT_LEAST_32_RANDOM_CHARACTERS")]),
        "AUTH_JWT_SECRET is still the template's placeholder"
    );
    assert!(error(&[("SMTP_SECURITY", "none")]).starts_with("SMTP_SECURITY"));
    assert!(error(&[("EMAIL_FROM", "not an address")]).starts_with("EMAIL_FROM"));
    // The app's host alone doesn't cover the back office's.
    assert!(error(&[("WEBAUTHN_RP_ID", "app.paccradle.com")]).starts_with("WEBAUTHN_RP_ID"));
    assert!(error(&[("WEBAUTHN_RP_ID", "example.com")]).starts_with("WEBAUTHN_RP_ID"));
    assert!(error(&[("WEBAUTHN_RP_ID", "ccradle.com")]).starts_with("WEBAUTHN_RP_ID"));
}
