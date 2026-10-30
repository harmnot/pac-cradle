import { decodeBase64url, encodeBase64url } from "@/lib/base64url";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";

/**
 * Passkeys in the browser (WebAuthn). The API speaks the W3C JSON shapes, so options go
 * through `PublicKeyCredential.parse…FromJSON` and answers through `credential.toJSON()`,
 * with a small base64url fallback for browsers that lack them (MILESTONES M1-E1).
 */

/** `AuthenticationResponseJSON`, what a passkey sign-in sends. */
export interface AuthenticationResponse {
  readonly id: string;
  readonly rawId: string;
  readonly type: "public-key";
  readonly authenticatorAttachment: string | null;
  readonly clientExtensionResults: AuthenticationExtensionsClientOutputs;
  readonly response: {
    readonly clientDataJSON: string;
    readonly authenticatorData: string;
    readonly signature: string;
    readonly userHandle: string | null;
  };
}

/** `RegistrationResponseJSON`, what adding a passkey sends. */
export interface RegistrationResponse {
  readonly id: string;
  readonly rawId: string;
  readonly type: "public-key";
  readonly authenticatorAttachment: string | null;
  readonly clientExtensionResults: AuthenticationExtensionsClientOutputs;
  readonly response: {
    readonly clientDataJSON: string;
    readonly attestationObject: string;
    readonly transports: readonly string[];
  };
}

/** Why a passkey ceremony ended without a credential. */
export const PasskeyFailure = {
  /** Cancelled, timed out, or no passkey for this site on the device (`NotAllowedError`). */
  NotAllowed: "not_allowed",
  /** The device already holds one of the account's passkeys (`InvalidStateError`). */
  AlreadyOnDevice: "already_on_device",
  /** Another ceremony took over (`AbortError`): ignore it. */
  Aborted: "aborted",
  /** The browser can't do passkeys, or the options couldn't be read. */
  Unsupported: "unsupported",
  /** Anything else the browser threw. */
  Other: "other",
} as const;

export type PasskeyFailure = (typeof PasskeyFailure)[keyof typeof PasskeyFailure];

/** The outcome of a ceremony in the browser. */
export type PasskeyOutcome<T> =
  | { readonly kind: "ok"; readonly credential: T }
  | { readonly kind: "failed"; readonly failure: PasskeyFailure };

/** How a sign-in ceremony asks: a dialog, or the email field's autofill. */
export const Mediation = {
  Modal: "optional",
  Autofill: "conditional",
} as const;

export type Mediation = (typeof Mediation)[keyof typeof Mediation];

const FAILURE_BY_ERROR_NAME: Readonly<Record<string, PasskeyFailure>> = {
  NotAllowedError: PasskeyFailure.NotAllowed,
  InvalidStateError: PasskeyFailure.AlreadyOnDevice,
  AbortError: PasskeyFailure.Aborted,
  NotSupportedError: PasskeyFailure.Unsupported,
  SecurityError: PasskeyFailure.Unsupported,
};

/** Whether this browser has WebAuthn at all. */
export function passkeysSupported(): boolean {
  return typeof window !== "undefined" && typeof window.PublicKeyCredential === "function";
}

/** Whether this device can make a passkey itself (Touch ID, Windows Hello, a phone's lock). */
export async function canMakePasskeyHere(): Promise<boolean> {
  if (!passkeysSupported()) {
    return false;
  }
  return PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable().catch(() => false);
}

/** Whether passkeys can be offered in the email field's autofill. */
export async function autofillSupported(): Promise<boolean> {
  if (!passkeysSupported() || typeof PublicKeyCredential.isConditionalMediationAvailable !== "function") {
    return false;
  }
  return PublicKeyCredential.isConditionalMediationAvailable().catch(() => false);
}

function failureOf(error: unknown): PasskeyFailure {
  if (error instanceof DOMException) {
    return FAILURE_BY_ERROR_NAME[error.name] ?? PasskeyFailure.Other;
  }
  return PasskeyFailure.Other;
}

function bytes(text: string): Maybe<Uint8Array<ArrayBuffer>> {
  return decodeBase64url(text);
}

function descriptors(
  list: readonly PublicKeyCredentialDescriptorJSON[] | undefined,
): Maybe<PublicKeyCredentialDescriptor[]> {
  const decoded = (list ?? []).map((item) => ({ item, id: bytes(item.id) }));
  if (decoded.some(({ id }) => id.kind === "none")) {
    return none;
  }
  return some(
    decoded.map(({ item, id }) => ({
      type: "public-key",
      id: id.kind === "some" ? id.value : new Uint8Array(),
      transports: item.transports as AuthenticatorTransport[] | undefined,
    })),
  );
}

/** Request options from their JSON, by hand. */
function requestOptions(json: PublicKeyCredentialRequestOptionsJSON): Maybe<PublicKeyCredentialRequestOptions> {
  if (typeof PublicKeyCredential.parseRequestOptionsFromJSON === "function") {
    return some(PublicKeyCredential.parseRequestOptionsFromJSON(json));
  }
  const challenge = bytes(json.challenge);
  const allow = descriptors(json.allowCredentials);
  if (challenge.kind === "none" || allow.kind === "none") {
    return none;
  }
  return some({
    challenge: challenge.value,
    rpId: json.rpId,
    timeout: json.timeout,
    userVerification: json.userVerification as UserVerificationRequirement | undefined,
    allowCredentials: allow.value,
  });
}

/** Creation options from their JSON, by hand. */
function creationOptions(json: PublicKeyCredentialCreationOptionsJSON): Maybe<PublicKeyCredentialCreationOptions> {
  if (typeof PublicKeyCredential.parseCreationOptionsFromJSON === "function") {
    return some(PublicKeyCredential.parseCreationOptionsFromJSON(json));
  }
  const challenge = bytes(json.challenge);
  const userId = bytes(json.user.id);
  const exclude = descriptors(json.excludeCredentials);
  if (challenge.kind === "none" || userId.kind === "none" || exclude.kind === "none") {
    return none;
  }
  return some({
    challenge: challenge.value,
    rp: json.rp,
    user: { id: userId.value, name: json.user.name, displayName: json.user.displayName },
    pubKeyCredParams: json.pubKeyCredParams,
    timeout: json.timeout,
    excludeCredentials: exclude.value,
    authenticatorSelection: json.authenticatorSelection,
    attestation: json.attestation as AttestationConveyancePreference | undefined,
    extensions: json.extensions as AuthenticationExtensionsClientInputs | undefined,
  });
}

function hasToJson(credential: PublicKeyCredential): boolean {
  return typeof credential.toJSON === "function";
}

function authenticationJson(credential: PublicKeyCredential): AuthenticationResponse {
  if (hasToJson(credential)) {
    return credential.toJSON() as AuthenticationResponse;
  }
  const response = credential.response as AuthenticatorAssertionResponse;
  return {
    id: credential.id,
    rawId: encodeBase64url(credential.rawId),
    type: "public-key",
    authenticatorAttachment: credential.authenticatorAttachment,
    clientExtensionResults: credential.getClientExtensionResults(),
    response: {
      clientDataJSON: encodeBase64url(response.clientDataJSON),
      authenticatorData: encodeBase64url(response.authenticatorData),
      signature: encodeBase64url(response.signature),
      userHandle: response.userHandle === null ? null : encodeBase64url(response.userHandle),
    },
  };
}

function registrationJson(credential: PublicKeyCredential): RegistrationResponse {
  if (hasToJson(credential)) {
    return credential.toJSON() as RegistrationResponse;
  }
  const response = credential.response as AuthenticatorAttestationResponse;
  return {
    id: credential.id,
    rawId: encodeBase64url(credential.rawId),
    type: "public-key",
    authenticatorAttachment: credential.authenticatorAttachment,
    clientExtensionResults: credential.getClientExtensionResults(),
    response: {
      clientDataJSON: encodeBase64url(response.clientDataJSON),
      attestationObject: encodeBase64url(response.attestationObject),
      transports: response.getTransports(),
    },
  };
}

/** Asks the browser for a passkey to sign in with. */
export async function getPasskey({
  options,
  mediation,
  signal,
}: {
  options: PublicKeyCredentialRequestOptionsJSON;
  mediation: Mediation;
  signal: AbortSignal;
}): Promise<PasskeyOutcome<AuthenticationResponse>> {
  if (!passkeysSupported()) {
    return { kind: "failed", failure: PasskeyFailure.Unsupported };
  }
  const publicKey = requestOptions(options);
  if (publicKey.kind === "none") {
    return { kind: "failed", failure: PasskeyFailure.Unsupported };
  }
  try {
    const credential = await navigator.credentials.get({
      publicKey: publicKey.value,
      mediation,
      signal,
    });
    if (!(credential instanceof PublicKeyCredential)) {
      return { kind: "failed", failure: PasskeyFailure.NotAllowed };
    }
    return { kind: "ok", credential: authenticationJson(credential) };
  } catch (error) {
    return { kind: "failed", failure: failureOf(error) };
  }
}

/** Asks the browser to make a passkey for the signed-in account. */
export async function makePasskey(
  options: PublicKeyCredentialCreationOptionsJSON,
): Promise<PasskeyOutcome<RegistrationResponse>> {
  if (!passkeysSupported()) {
    return { kind: "failed", failure: PasskeyFailure.Unsupported };
  }
  const publicKey = creationOptions(options);
  if (publicKey.kind === "none") {
    return { kind: "failed", failure: PasskeyFailure.Unsupported };
  }
  try {
    const credential = await navigator.credentials.create({ publicKey: publicKey.value });
    if (!(credential instanceof PublicKeyCredential)) {
      return { kind: "failed", failure: PasskeyFailure.NotAllowed };
    }
    return { kind: "ok", credential: registrationJson(credential) };
  } catch (error) {
    return { kind: "failed", failure: failureOf(error) };
  }
}

/** A name for a passkey made on this device, from the browser's platform hint. */
export function deviceName(): string {
  const platform = navigator.userAgent;
  const known: ReadonlyArray<readonly [RegExp, string]> = [
    [/iPhone/, "iPhone"],
    [/iPad/, "iPad"],
    [/Android/, "Android"],
    [/Mac OS X|Macintosh/, "Mac"],
    [/Windows/, "Windows"],
    [/CrOS/, "Chromebook"],
    [/Linux/, "Linux"],
  ];
  const match = known.find(([pattern]) => pattern.test(platform));
  return match ? match[1] : t("passkey.thisDevice");
}
