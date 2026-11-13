import { create } from '@bufbuild/protobuf';
import { IdentifySchema, ClientFrameSchema } from './src/lib/proto/dionysus/chat/v1/message_pb.js';
import { UserIDSchema } from './src/lib/proto/dionysus/user/v1/user_pb.js';
import { UUIDSchema } from './src/lib/proto/dionysus/common/v1/asset_pb.js';
import { toBinary } from '@bufbuild/protobuf';

// Test the userId generation like the chat store does
const userId = crypto.getRandomValues(new Uint8Array(16));
userId[6] = (userId[6] & 0x0f) | 0x40;
userId[8] = (userId[8] & 0x3f) | 0x80;

console.log('Generated userId:', Array.from(userId));

const identify = create(IdentifySchema, {
  userId: create(UserIDSchema, { value: create(UUIDSchema, { value: userId }) }),
  token: 'demo-token',
});

const frame = create(ClientFrameSchema, { payload: { case: 'identify', value: identify } });
const bytes = toBinary(ClientFrameSchema, frame);

console.log('Frame bytes length:', bytes.length);
console.log('Frame bytes:', Array.from(bytes));

// Also test with fixed array like the working test
const fixedId = new Uint8Array([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
const identify2 = create(IdentifySchema, {
  userId: create(UserIDSchema, { value: create(UUIDSchema, { value: fixedId }) }),
  token: 'test-token',
});
const frame2 = create(ClientFrameSchema, { payload: { case: 'identify', value: identify2 } });
const bytes2 = toBinary(ClientFrameSchema, frame2);
console.log('Fixed frame bytes:', Array.from(bytes2));
