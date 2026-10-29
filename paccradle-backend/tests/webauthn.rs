//! Passkey checks (M1-D3), against a software authenticator: what registration and sign-in
//! accept, and each thing they refuse.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::authenticator::{
    ATTESTED, KeyKind, SoftAuthenticator, cbor_bytes, cbor_map, cbor_text,
};
use paccradle_backend::shared::webauthn::{
    self, Authentication, CoseKey, Expected, NewCredential, Registration, StoredCredential,
    WebauthnError,
};

const ORIGIN: &str = "http://localhost:3000";
const CHALLENGE: [u8; 32] = [9; 32];
const USER: [u8; 16] = [7; 16];

fn expected() -> Expected<'static> {
    Expected {
        challenge: &CHALLENGE,
        origin: ORIGIN,
        rp_id: "localhost",
    }
}

fn register(device: &SoftAuthenticator) -> Result<NewCredential, WebauthnError> {
    let (client_data_json, attestation_object) = device.registration_parts(&CHALLENGE);
    webauthn::verify_registration(
        &expected(),
        &Registration {
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        },
    )
}

/// A sign-in by `device` with the credential registration stored, after `tweak` has had a go at
/// the response's parts.
fn sign_in(
    device: &mut SoftAuthenticator,
    stored: &NewCredential,
    tweak: impl FnOnce(&mut Vec<u8>, &mut Vec<u8>, &mut Vec<u8>),
) -> Result<webauthn::Assertion, WebauthnError> {
    let (mut client_data, mut auth_data, mut signature) = device.assertion_parts(&CHALLENGE);
    tweak(&mut client_data, &mut auth_data, &mut signature);
    let stored = StoredCredential {
        public_key: &stored.public_key,
        sign_count: stored.sign_count,
        backup_eligible: stored.backup_eligible,
        user_handle: &USER,
    };
    let response = Authentication {
        client_data_json: &client_data,
        authenticator_data: &auth_data,
        signature: &signature,
        user_handle: Some(&USER),
    };
    webauthn::verify_authentication(&expected(), (&stored, &response))
}

#[test]
fn each_key_type_registers_and_signs_in() {
    for (kind, algorithm) in [
        (KeyKind::P256, -7),
        (KeyKind::Ed25519, -8),
        (KeyKind::Rsa, -257),
    ] {
        let mut device = SoftAuthenticator::new(kind, ORIGIN);
        let stored = register(&device).unwrap();

        assert_eq!(stored.credential_id, device.credential_id, "{kind:?}");
        assert_eq!(stored.algorithm, algorithm);
        assert_eq!(stored.public_key, device.cose_key());
        assert!(stored.backup_eligible && stored.backed_up);
        assert_eq!(stored.aaguid, None, "all zeros means none");
        assert_eq!(
            CoseKey::from_bytes(&stored.public_key).unwrap().algorithm(),
            algorithm
        );

        let assertion = sign_in(&mut device, &stored, |_, _, _| {}).unwrap();
        assert_eq!(assertion.sign_count, 0);
        assert!(assertion.backed_up);
    }
}

#[test]
fn registration_refuses_another_ceremony_challenge_origin_or_site() {
    let device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    let (_, attestation) = device.registration_parts(&CHALLENGE);
    let refuse = |client_data: Vec<u8>, attestation: &[u8]| {
        webauthn::verify_registration(
            &expected(),
            &Registration {
                client_data_json: &client_data,
                attestation_object: attestation,
            },
        )
        .unwrap_err()
    };

    assert_eq!(
        refuse(device.client_data("webauthn.get", &CHALLENGE), &attestation).0,
        "client data is for another ceremony"
    );
    assert_eq!(
        refuse(
            device.client_data("webauthn.create", &[1; 32]),
            &attestation
        )
        .0,
        "client data has another challenge"
    );
    let mut elsewhere = SoftAuthenticator::new(KeyKind::P256, "https://evil.example");
    elsewhere.credential_id.clone_from(&device.credential_id);
    assert_eq!(
        refuse(
            elsewhere.client_data("webauthn.create", &CHALLENGE),
            &attestation
        )
        .0,
        "client data comes from another origin"
    );
    let framed = serde_json::to_vec(&serde_json::json!({
        "type": "webauthn.create",
        "challenge": paccradle_backend::shared::base64url::encode(&CHALLENGE),
        "origin": ORIGIN,
        "crossOrigin": true,
    }))
    .unwrap();
    assert_eq!(
        refuse(framed, &attestation).0,
        "client data comes from a frame"
    );

    let mut other_site = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    other_site.rp_id = "evil.example".to_owned();
    assert_eq!(
        register(&other_site).unwrap_err().0,
        "passkey is for another site (RP ID hash)"
    );
}

#[test]
fn registration_needs_presence_verification_and_consistent_backup_flags() {
    let cases = [
        (0x04 | 0x08 | 0x10, "user wasn't present"),
        (0x01 | 0x08 | 0x10, "user wasn't verified"),
        (0x01 | 0x04 | 0x10, "backed up but not backup eligible"),
    ];
    for (flags, reason) in cases {
        let mut device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
        device.flags = flags;
        assert_eq!(register(&device).unwrap_err().0, reason);
    }
    // A device-bound passkey (no backup) is fine.
    let mut bound = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    bound.flags = 0x01 | 0x04;
    let stored = register(&bound).unwrap();
    assert!(!stored.backup_eligible && !stored.backed_up);
}

#[test]
fn registration_refuses_broken_attestation_objects() {
    let device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    let client_data = device.client_data("webauthn.create", &CHALLENGE);
    let auth_data = device.authenticator_data(true);
    let try_with = |attestation: Vec<u8>| {
        webauthn::verify_registration(
            &expected(),
            &Registration {
                client_data_json: &client_data,
                attestation_object: &attestation,
            },
        )
    };
    // No authData.
    assert!(try_with(cbor_map(&[(cbor_text("fmt"), cbor_text("none"))])).is_err());
    // Bytes after the credential.
    let mut padded = auth_data.clone();
    padded.push(0);
    let with = |auth_data: &[u8]| {
        cbor_map(&[
            (cbor_text("fmt"), cbor_text("none")),
            (cbor_text("attStmt"), cbor_map(&[])),
            (cbor_text("authData"), cbor_bytes(auth_data)),
        ])
    };
    assert_eq!(
        try_with(with(&padded)).unwrap_err().0,
        "authenticator data has extra bytes"
    );
    // No credential (attested flag off).
    let mut flagless = auth_data.clone();
    flagless[32] &= !ATTESTED;
    flagless.truncate(37);
    assert_eq!(
        try_with(with(&flagless)).unwrap_err().0,
        "no new credential in the authenticator data"
    );
    // Any attestation format is read the same way: only authData counts.
    let packed = cbor_map(&[
        (cbor_text("fmt"), cbor_text("packed")),
        (
            cbor_text("attStmt"),
            cbor_map(&[(cbor_text("alg"), vec![0x26])]),
        ),
        (cbor_text("authData"), cbor_bytes(&auth_data)),
    ]);
    assert!(try_with(packed).is_ok());
}

#[test]
fn sign_in_refuses_a_bad_signature_another_user_or_another_ceremony() {
    let mut device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    let stored = register(&device).unwrap();

    let error = sign_in(&mut device, &stored, |_, _, signature| {
        let last = signature.len() - 1;
        signature[last] ^= 1;
    });
    assert_eq!(error.unwrap_err().0, "signature doesn't match the passkey");

    let error = sign_in(&mut device, &stored, |client_data, _, _| {
        *client_data = SoftAuthenticator::new(KeyKind::P256, ORIGIN)
            .client_data("webauthn.create", &CHALLENGE);
    });
    assert_eq!(error.unwrap_err().0, "client data is for another ceremony");

    // Data signed, then changed: the signature no longer covers it.
    let error = sign_in(&mut device, &stored, |_, auth_data, _| auth_data[33] = 0xff);
    assert_eq!(error.unwrap_err().0, "signature doesn't match the passkey");

    // Another device's key can't sign for this credential.
    let mut other = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    let error = sign_in(&mut other, &stored, |_, _, _| {});
    assert_eq!(error.unwrap_err().0, "signature doesn't match the passkey");

    let (client_data, auth_data, signature) = device.assertion_parts(&CHALLENGE);
    let stored_for = |handle: &'static [u8]| StoredCredential {
        public_key: &stored.public_key,
        sign_count: 0,
        backup_eligible: true,
        user_handle: handle,
    };
    for user_handle in [None, Some(&[8u8; 16][..])] {
        let response = Authentication {
            client_data_json: &client_data,
            authenticator_data: &auth_data,
            signature: &signature,
            user_handle,
        };
        let error = webauthn::verify_authentication(&expected(), (&stored_for(&USER), &response));
        assert_eq!(
            error.unwrap_err().0,
            "user handle doesn't match the passkey's owner"
        );
    }
}

#[test]
fn sign_in_needs_a_rising_counter_once_one_is_kept() {
    let mut device = SoftAuthenticator::new(KeyKind::Ed25519, ORIGIN);
    device.counting = true;
    device.sign_count = 10;
    let mut stored = register(&device).unwrap();
    assert_eq!(stored.sign_count, 10);

    let assertion = sign_in(&mut device, &stored, |_, _, _| {}).unwrap();
    assert_eq!(assertion.sign_count, 11);
    stored.sign_count = assertion.sign_count;

    // A clone still at 11 (or behind) is refused.
    device.counting = false;
    let error = sign_in(&mut device, &stored, |_, _, _| {});
    assert_eq!(error.unwrap_err().0, "signature counter didn't increase");
}

#[test]
fn sign_in_refuses_a_changed_backup_eligibility_or_missing_verification() {
    let mut device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    let stored = register(&device).unwrap();

    device.flags = 0x01 | 0x04;
    let error = sign_in(&mut device, &stored, |_, _, _| {});
    assert_eq!(error.unwrap_err().0, "backup eligibility changed");

    device.flags = 0x01 | 0x08 | 0x10;
    let error = sign_in(&mut device, &stored, |_, _, _| {});
    assert_eq!(error.unwrap_err().0, "user wasn't verified");
}

#[test]
fn cose_keys_must_be_whole_and_of_an_accepted_algorithm() {
    let device = SoftAuthenticator::new(KeyKind::P256, ORIGIN);
    assert!(CoseKey::from_bytes(&device.cose_key()).is_ok());

    let int = |n: i64| common::authenticator::cbor_int(n);
    let refused = [
        // ES256 with a 31-byte x.
        cbor_map(&[
            (int(1), int(2)),
            (int(3), int(-7)),
            (int(-1), int(1)),
            (int(-2), cbor_bytes(&[1; 31])),
            (int(-3), cbor_bytes(&[1; 32])),
        ]),
        // EC2 on P-384 (crv 2).
        cbor_map(&[
            (int(1), int(2)),
            (int(3), int(-7)),
            (int(-1), int(2)),
            (int(-2), cbor_bytes(&[1; 32])),
            (int(-3), cbor_bytes(&[1; 32])),
        ]),
        // ES384, not offered.
        cbor_map(&[(int(1), int(2)), (int(3), int(-35))]),
        // A 1024-bit RSA key.
        cbor_map(&[
            (int(1), int(3)),
            (int(3), int(-257)),
            (int(-1), cbor_bytes(&[0xc1; 128])),
            (int(-2), cbor_bytes(&[1, 0, 1])),
        ]),
    ];
    for key in refused {
        assert!(CoseKey::from_bytes(&key).is_err(), "{key:02x?}");
    }
}
