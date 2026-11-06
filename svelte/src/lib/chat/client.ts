import type {
  ClientFrame,
  ServerFrame,
  Identify,
  JoinChannel,
  SendMessage,
  EditMessage,
  Ping,
  Message,
  MessageEdited,
  UserJoined,
  UserLeft,
  ChannelSnapshot,
  Error,
  Pong,
} from '$lib/proto/dionysus/chat/v1/message_pb';
import {
  ClientFrameSchema,
  ServerFrameSchema,
  IdentifySchema,
  JoinChannelSchema,
  SendMessageSchema,
  EditMessageSchema,
  PingSchema,
  MessageIDSchema,
} from '$lib/proto/dionysus/chat/v1/message_pb';
import type { UserID } from '$lib/proto/dionysus/user/v1/user_pb';
import { UserIDSchema } from '$lib/proto/dionysus/user/v1/user_pb';
import type { RoomID } from '$lib/proto/dionysus/room/v1/room_pb';
import { RoomIDSchema } from '$lib/proto/dionysus/room/v1/room_pb';
import type { UUID } from '$lib/proto/dionysus/common/v1/asset_pb';
import { UUIDSchema } from '$lib/proto/dionysus/common/v1/asset_pb';
import { create, toBinary, fromBinary } from '@bufbuild/protobuf';

export type ChatEvent =
  | { type: 'connected' }
  | { type: 'disconnected' }
  | { type: 'error'; error: Error }
  | { type: 'message'; message: Message }
  | { type: 'edited'; edited: MessageEdited }
  | { type: 'joined'; joined: UserJoined }
  | { type: 'left'; left: UserLeft }
  | { type: 'snapshot'; snapshot: ChannelSnapshot }
  | { type: 'pong' };

type EventHandler = (event: ChatEvent) => void;

export class ChatClient {
  private ws: WebSocket | null = null;
  private url: string;
  private handlers: Set<EventHandler> = new Set();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setTimeout> | null = null;
  private userId: Uint8Array | null = null;
  private token: string = '';
  private currentChannelId: Uint8Array | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;

  constructor(url: string = 'ws://localhost:9001') {
    this.url = url;
  }

  connect(userId: Uint8Array, token: string): Promise<void> {
    this.userId = userId;
    this.token = token;
    this.reconnectAttempts = 0;
    return this.doConnect();
  }

  private doConnect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
          console.log('[ChatClient] Connected');
          this.reconnectAttempts = 0;
          this.sendIdentify();
          this.startPing();
          this.emit({ type: 'connected' });
          resolve();
        };

        this.ws.onclose = () => {
          console.log('[ChatClient] Disconnected');
          this.stopPing();
          this.emit({ type: 'disconnected' });
          this.scheduleReconnect();
        };

        this.ws.onerror = (event) => {
          console.error('[ChatClient] Error:', event);
          reject(new globalThis.Error('WebSocket connection failed'));
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data);
        };
      } catch (e) {
        reject(e);
      }
    });
  }

  private scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('[ChatClient] Max reconnect attempts reached');
      return;
    }
    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts - 1), 30000);
    console.log(`[ChatClient] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    this.reconnectTimer = setTimeout(() => {
      this.doConnect().catch(console.error);
    }, delay);
  }

  private startPing() {
    this.pingTimer = setInterval(() => {
      this.sendPing();
    }, 30000);
  }

  private stopPing() {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private sendIdentify() {
    if (!this.userId) return;
    const identify: Identify = create(IdentifySchema, {
      userId: create(UserIDSchema, { value: create(UUIDSchema, { value: this.userId }) }),
      token: this.token,
    });
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'identify', value: identify } });
    this.sendFrame(frame);
  }

  private sendPing() {
    const ping: Ping = create(PingSchema, {});
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'ping', value: ping } });
    this.sendFrame(frame);
  }

  joinChannel(channelId: Uint8Array) {
    this.currentChannelId = channelId;
    const join: JoinChannel = create(JoinChannelSchema, {
      channelId: create(RoomIDSchema, { value: create(UUIDSchema, { value: channelId }) }),
    });
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'join', value: join } });
    this.sendFrame(frame);
  }

  leaveChannel(channelId: Uint8Array) {
    const leave: LeaveChannel = create(JoinChannelSchema, {
      channelId: create(RoomIDSchema, { value: create(UUIDSchema, { value: channelId }) }),
    });
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'leave', value: leave } });
    this.sendFrame(frame);
  }

  sendMessage(channelId: Uint8Array, text: string, repliesTo?: { value?: UUID }) {
    const send: SendMessage = create(SendMessageSchema, {
      roomId: create(RoomIDSchema, { value: create(UUIDSchema, { value: channelId }) }),
      message: text,
      repliesTo: repliesTo ? create(MessageIDSchema, repliesTo) : undefined,
    });
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'send', value: send } });
    this.sendFrame(frame);
  }

  editMessage(messageId: Uint8Array, newText: string) {
    const edit: EditMessage = create(EditMessageSchema, {
      id: create(MessageIDSchema, { value: create(UUIDSchema, { value: messageId }) }),
      newMessage: newText,
    });
    const frame: ClientFrame = create(ClientFrameSchema, { payload: { case: 'edit', value: edit } });
    this.sendFrame(frame);
  }

  private sendFrame(frame: ClientFrame) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[ChatClient] Not connected, dropping message');
      return;
    }
    const bytes = toBinary(ClientFrameSchema, frame);
    this.ws.send(bytes);
  }

  private handleMessage(data: ArrayBuffer) {
    try {
      const frame = fromBinary(ServerFrameSchema, new Uint8Array(data));
      this.processServerFrame(frame);
    } catch (e) {
      console.error('[ChatClient] Failed to decode message:', e);
    }
  }

  private processServerFrame(frame: ServerFrame) {
    if (!frame.payload) return;

    switch (frame.payload.case) {
      case 'message':
        this.emit({ type: 'message', message: frame.payload.value });
        break;
      case 'edited':
        this.emit({ type: 'edited', edited: frame.payload.value });
        break;
      case 'joined':
        this.emit({ type: 'joined', joined: frame.payload.value });
        break;
      case 'left':
        this.emit({ type: 'left', left: frame.payload.value });
        break;
      case 'snapshot':
        this.emit({ type: 'snapshot', snapshot: frame.payload.value });
        break;
      case 'pong':
        this.emit({ type: 'pong' });
        break;
      case 'error':
        this.emit({ type: 'error', error: frame.payload.value });
        break;
    }
  }

  on(handler: EventHandler) {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  private emit(event: ChatEvent) {
    for (const handler of this.handlers) {
      try {
        handler(event);
      } catch (e) {
        console.error('[ChatClient] Handler error:', e);
      }
    }
  }

  disconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.stopPing();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}
