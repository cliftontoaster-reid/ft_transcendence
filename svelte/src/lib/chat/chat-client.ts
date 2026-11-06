// connecte un client à un serveur de chat via WebSocket et gère l’envoi et la réception de messages

export type ChatMessage = { sender: string; text: string };
export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

type ProtoField = { number: number; wireType: number; bytes?: Uint8Array }; // Représente un champ d’un message Protobuf

type ChatClientOptions = {
	channelId: string;
	onStatus: (status: ConnectionStatus) => void;
	onError: (message: string) => void;
	onMessage: (message: ChatMessage) => void;
	onSnapshot: (messages: ChatMessage[]) => void;
};

export type ChatClient = {
	send: (text: string) => boolean;
	close: () => void;
};

const DEFAULT_CHANNEL_ID = '00000000-0000-0000-0000-000000000001';
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function encodeVarint(value: number): number[] {
	const result: number[] = [];
	while (value > 0x7f) {
		result.push((value & 0x7f) | 0x80);
		value >>>= 7;
	}
	result.push(value);
	return result;
}

function encodeBytesField(number: number, value: Uint8Array): Uint8Array {
	return Uint8Array.from([
		...encodeVarint((number << 3) | 2),
		...encodeVarint(value.length),
		...value
	]);
}

function encodeStringField(number: number, value: string): Uint8Array {
	return encodeBytesField(number, encoder.encode(value));
}

function readVarint(data: Uint8Array, start: number): { value: number; next: number } {
	let value = 0;
	let shift = 0;
	let offset = start;
	while (offset < data.length && shift < 35) {
		const byte = data[offset++];
		value |= (byte & 0x7f) << shift;
		if ((byte & 0x80) === 0) return { value, next: offset };
		shift += 7;
	}
	throw new Error('Trame Protobuf invalide.');
}

function decodeFields(data: Uint8Array): ProtoField[] {
	const fields: ProtoField[] = [];
	let offset = 0;
	while (offset < data.length) {
		const tag = readVarint(data, offset);
		offset = tag.next;
		const number = tag.value >>> 3;
		const wireType = tag.value & 7;
		if (wireType === 2) {
			const length = readVarint(data, offset);
			offset = length.next;
			const end = offset + length.value;
			if (end > data.length) throw new Error('Trame Protobuf tronquée.');
			fields.push({ number, wireType, bytes: data.subarray(offset, end) });
			offset = end;
		} else if (wireType === 0) {
			offset = readVarint(data, offset).next;
		} else if (wireType === 1) {
			offset += 8;
		} else if (wireType === 5) {
			offset += 4;
		} else {
			throw new Error('Type de champ Protobuf inconnu.');
		}
		if (offset > data.length) throw new Error('Trame Protobuf tronquée.');
	}
	return fields;
}

function firstBytes(fields: ProtoField[], number: number): Uint8Array | undefined {
	return fields.find((field) => field.number === number && field.wireType === 2)?.bytes;
}

function uuidToBytes(value: string): Uint8Array {
	if (!UUID_PATTERN.test(value)) throw new Error('L’identifiant du salon doit être un UUID valide.');
	return Uint8Array.from(value.replaceAll('-', '').match(/.{2}/g)!.map((byte) => parseInt(byte, 16)));
}

function bytesToHex(value: Uint8Array): string {
	return Array.from(value, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function getLocalUserId(): Uint8Array {
	const storageKey = 'dionysus-chat-user-id';
	let userId = localStorage.getItem(storageKey);
	if (!userId || !UUID_PATTERN.test(userId)) {
		userId = crypto.randomUUID();
		localStorage.setItem(storageKey, userId);
	}
	return uuidToBytes(userId);
}

function encodeClientFrame(fieldNumber: number, payload: Uint8Array): Uint8Array {
	return encodeBytesField(fieldNumber, payload);
}

function encodeIdentifyFrame(userId: Uint8Array): Uint8Array {
	const uuid = encodeBytesField(1, userId);
	const user = encodeBytesField(1, uuid);
	return encodeClientFrame(1, encodeBytesField(1, user));
}

function encodeJoinFrame(roomId: Uint8Array): Uint8Array {
	const uuid = encodeBytesField(1, roomId);
	const room = encodeBytesField(1, uuid);
	return encodeClientFrame(2, encodeBytesField(1, room));
}

function encodeSendFrame(roomId: Uint8Array, text: string): Uint8Array {
	const uuid = encodeBytesField(1, roomId);
	const room = encodeBytesField(1, uuid);
	const send = new Uint8Array([...encodeBytesField(1, room), ...encodeStringField(2, text)]);
	return encodeClientFrame(4, send);
}

function sendBinaryFrame(socket: WebSocket, frame: Uint8Array) {
	const buffer = new ArrayBuffer(frame.byteLength);
	new Uint8Array(buffer).set(frame);
	socket.send(buffer);
}

function decodeMessage(data: Uint8Array, currentUserId: Uint8Array): ChatMessage {
	const messageFields = decodeFields(data);
	const senderId = firstBytes(messageFields, 2);
	const userId = senderId ? firstBytes(decodeFields(senderId), 1) : undefined;
	const uuid = userId ? firstBytes(decodeFields(userId), 1) : undefined;
	const sender = uuid
		? bytesToHex(uuid) === bytesToHex(currentUserId)
			? 'Moi'
			: `Utilisateur ${bytesToHex(uuid).slice(0, 8)}`
		: 'Utilisateur';
	return { sender, text: decoder.decode(firstBytes(messageFields, 4) ?? new Uint8Array()) };
}

export function connectToChat(options: ChatClientOptions): ChatClient {
	try {
		const currentUserId = getLocalUserId();
		const roomId = uuidToBytes(options.channelId || DEFAULT_CHANNEL_ID);
		const socketUrl =
			import.meta.env.VITE_CHAT_WS_URL ??
			`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.hostname}:9001`;
		const socket = new WebSocket(socketUrl);
		socket.binaryType = 'arraybuffer';

		socket.addEventListener('open', () => {
			options.onStatus('connected');
			sendBinaryFrame(socket, encodeIdentifyFrame(currentUserId));
			sendBinaryFrame(socket, encodeJoinFrame(roomId));
		});

		socket.addEventListener('message', (event: MessageEvent) => {
			if (!(event.data instanceof ArrayBuffer)) {
				options.onError('Le serveur a envoyé une donnée au format inattendu.');
				return;
			}
			try {
				for (const field of decodeFields(new Uint8Array(event.data))) {
					if (field.wireType !== 2 || !field.bytes) continue;
					if (field.number === 1) {
						options.onMessage(decodeMessage(field.bytes, currentUserId));
					} else if (field.number === 5) {
						const snapshot = decodeFields(field.bytes);
						options.onSnapshot(
							snapshot
								.filter((item) => item.number === 2 && item.bytes)
								.map((item) => decodeMessage(item.bytes!, currentUserId))
						);
					} else if (field.number === 7) {
						const errorFields = decodeFields(field.bytes);
						const code = decoder.decode(firstBytes(errorFields, 1) ?? new Uint8Array());
						const message = decoder.decode(firstBytes(errorFields, 2) ?? new Uint8Array());
						options.onError(`${code}: ${message}`);
					}
				}
			} catch (error) {
				options.onError(error instanceof Error ? error.message : 'Réponse du serveur invalide.');
			}
		});

		socket.addEventListener('error', () => {
			options.onStatus('error');
			options.onError('Connexion au serveur de chat impossible.');
		});

		socket.addEventListener('close', () => options.onStatus('disconnected'));

		return {
			send(text) {
				if (socket.readyState !== WebSocket.OPEN) {
					options.onError('Le serveur de chat n’est pas connecté.');
					return false;
				}
				try {
					sendBinaryFrame(socket, encodeSendFrame(roomId, text));
					return true;
				} catch (error) {
					options.onError(error instanceof Error ? error.message : 'Envoi du message impossible.');
					return false;
				}
			},
			close() {
				if (socket.readyState < WebSocket.CLOSING) socket.close();
			}
		};
	} catch (error) {
		options.onStatus('error');
		options.onError(error instanceof Error ? error.message : 'Connexion au serveur impossible.');
		return { send: () => false, close: () => {} };
	}
}