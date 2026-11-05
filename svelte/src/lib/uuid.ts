/** Convert between the canonical UUID string and the 16 raw bytes used by `dionysus.common.v1.UUID`. */

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isUuid(value: string): boolean {
	return UUID_RE.test(value);
}

export function uuidToBytes(uuid: string): Uint8Array<ArrayBuffer> {
	if (!isUuid(uuid)) throw new Error(`invalid UUID: ${uuid}`);
	const hex = uuid.replaceAll('-', '');
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 16; i++) bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
	return bytes;
}

export function bytesToUuid(bytes: Uint8Array): string {
	if (bytes.length !== 16) return '';
	const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
	return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
