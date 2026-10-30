import { none, some, type Maybe } from "./maybe";

/**
 * Unpadded base64url (RFC 4648 §5), as WebAuthn's JSON shapes and the API use it. Only for
 * browsers without `PublicKeyCredential.parse…FromJSON` and `credential.toJSON()`.
 */

const PLUS = /\+/g;
const SLASH = /\//g;
const PADDING = /=+$/;
const DASH = /-/g;
const UNDERSCORE = /_/g;
const BASE64URL_SHAPE = /^[A-Za-z0-9_-]*$/;
const BLOCK = 4;

/** Encodes bytes as unpadded base64url. */
export function encodeBase64url(bytes: ArrayBuffer | ArrayBufferView): string {
  const view =
    bytes instanceof ArrayBuffer
      ? new Uint8Array(bytes)
      : new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const binary = Array.from(view, (byte) => String.fromCharCode(byte)).join("");
  return btoa(binary).replace(PLUS, "-").replace(SLASH, "_").replace(PADDING, "");
}

/** Decodes unpadded (or padded) base64url; `none` for anything else. */
export function decodeBase64url(text: string): Maybe<Uint8Array<ArrayBuffer>> {
  const unpadded = text.replace(PADDING, "");
  if (!BASE64URL_SHAPE.test(unpadded) || unpadded.length % BLOCK === 1) {
    return none;
  }
  const base64 = unpadded.replace(DASH, "+").replace(UNDERSCORE, "/");
  const padded = base64.padEnd(Math.ceil(base64.length / BLOCK) * BLOCK, "=");
  try {
    const binary = atob(padded);
    return some(Uint8Array.from(binary, (char) => char.charCodeAt(0)));
  } catch {
    return none;
  }
}
