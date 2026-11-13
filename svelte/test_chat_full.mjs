import { create } from '@bufbuild/protobuf';
import { 
  ClientFrameSchema, 
  IdentifySchema, 
  JoinChannelSchema,
  SendMessageSchema,
  ServerFrameSchema 
} from './src/lib/proto/dionysus/chat/v1/message_pb.js';
import { UserIDSchema } from './src/lib/proto/dionysus/user/v1/user_pb.js';
import { RoomIDSchema } from './src/lib/proto/dionysus/room/v1/room_pb.js';
import { UUIDSchema } from './src/lib/proto/dionysus/common/v1/asset_pb.js';
import { toBinary, fromBinary } from '@bufbuild/protobuf';

// Use the EXACT same userId generation as chat store
const userId = crypto.getRandomValues(new Uint8Array(16));
userId[6] = (userId[6] & 0x0f) | 0x40;
userId[8] = (userId[8] & 0x3f) | 0x80;

console.log('Using userId:', Array.from(userId));

const ws = new WebSocket('ws://localhost:9001');
ws.binaryType = 'arraybuffer';

ws.onopen = () => {
  console.log('Connected!');
  
  // Send Identify
  const identify = create(IdentifySchema, {
    userId: create(UserIDSchema, { value: create(UUIDSchema, { value: userId }) }),
    token: 'demo-token',
  });
  const frame = create(ClientFrameSchema, { payload: { case: 'identify', value: identify } });
  ws.send(toBinary(ClientFrameSchema, frame));
  
  // Join global channel - wait a bit like the store does
  setTimeout(() => {
    const join = create(JoinChannelSchema, {
      channelId: create(RoomIDSchema, { value: create(UUIDSchema, { value: new TextEncoder().encode('global') }) }),
    });
    const frame2 = create(ClientFrameSchema, { payload: { case: 'join', value: join } });
    ws.send(toBinary(ClientFrameSchema, frame2));
    console.log('Sent JoinChannel');
  }, 100);
  
  // Send message
  setTimeout(() => {
    const send = create(SendMessageSchema, {
      roomId: create(RoomIDSchema, { value: create(UUIDSchema, { value: new TextEncoder().encode('global') }) }),
      message: 'Hello from test!',
    });
    const frame3 = create(ClientFrameSchema, { payload: { case: 'send', value: send } });
    ws.send(toBinary(ClientFrameSchema, frame3));
    console.log('Sent message');
  }, 500);
};

ws.onmessage = (event) => {
  try {
    const frame = fromBinary(ServerFrameSchema, new Uint8Array(event.data));
    console.log('Received:', frame.payload?.case);
    if (frame.payload?.case === 'message') {
      console.log('Message:', frame.payload.value.message);
    } else if (frame.payload?.case === 'snapshot') {
      console.log('Snapshot messages:', frame.payload.value.recentMessages.length);
    } else if (frame.payload?.case === 'error') {
      console.log('ERROR:', frame.payload.value.code, frame.payload.value.message);
    }
  } catch (e) {
    console.error('Decode error:', e);
  }
};

ws.onclose = () => console.log('Closed');
ws.onerror = (e) => console.error('Error:', e);

setTimeout(() => ws.close(), 3000);
