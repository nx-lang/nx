/**
 * Source in the address: `#code=<payload>`, where the payload is the source's UTF-8 bytes,
 * compressed with raw DEFLATE, in unpadded base64url.
 *
 * <para>The encoding is the contract, not this code: the website builds its "Open in playground"
 * links with `node:zlib` at build time, and any site can do the same in a few lines. DEFLATE output
 * is not canonical across implementations, so what is promised is that any valid raw DEFLATE stream
 * decodes. `fixture.json`, made with `node:zlib`, holds both sides to it.</para>
 *
 * <para>The browser's `CompressionStream` and `DecompressionStream` do the work, so the playground
 * carries no compression library.</para>
 */

/** The fragment's key: `#code=…`. */
export const FRAGMENT_KEY = "code";

/** Why a payload could not be read. */
export class ShareDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ShareDecodeError";
  }
}

/** Encodes `source` as a payload. */
export async function encodeSource(source: string): Promise<string> {
  const bytes = new TextEncoder().encode(source);
  return toBase64Url(await transform(bytes, new CompressionStream("deflate-raw")));
}

/**
 * Decodes a payload back to the source.
 *
 * @throws ShareDecodeError when the payload is not base64url, does not inflate, or is not UTF-8.
 */
export async function decodeSource(payload: string): Promise<string> {
  const bytes = fromBase64Url(payload);
  let inflated: Uint8Array;
  try {
    inflated = await transform(bytes, new DecompressionStream("deflate-raw"));
  } catch {
    throw new ShareDecodeError("The link's source is not compressed data.");
  }
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(inflated);
  } catch {
    throw new ShareDecodeError("The link's source is not UTF-8 text.");
  }
}

/** The payload a fragment such as `#code=…` carries, or null when it carries none. */
export function payloadFromFragment(hash: string): string | null {
  const fragment = hash.startsWith("#") ? hash.slice(1) : hash;
  const prefix = `${FRAGMENT_KEY}=`;
  return fragment.startsWith(prefix) ? fragment.slice(prefix.length) : null;
}

/** The fragment for a payload, `#code=…`. */
export function fragmentForPayload(payload: string): string {
  return `#${FRAGMENT_KEY}=${payload}`;
}

export function toBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** @throws ShareDecodeError when `text` is not unpadded base64url. */
export function fromBase64Url(text: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]*$/.test(text) || text.length % 4 === 1) {
    throw new ShareDecodeError("The link's source is not base64url.");
  }
  const padded = text.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (text.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

async function transform(bytes: Uint8Array, stream: CompressionStream | DecompressionStream): Promise<Uint8Array> {
  const output = new Blob([bytes as BlobPart]).stream().pipeThrough(stream);
  return new Uint8Array(await new Response(output).arrayBuffer());
}
