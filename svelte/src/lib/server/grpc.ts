import { createClient } from '@connectrpc/connect';
import { createGrpcTransport } from '@connectrpc/connect-node';
import { env } from '$env/dynamic/private';
import { UserService } from '$lib/proto/dionysus/user/v1/user_pb';

// Server-side only: the browser never talks gRPC, it talks to SvelteKit.
const transport = createGrpcTransport({
	baseUrl: env.USER_SERVICE_URL ?? 'http://127.0.0.1:50052'
});

export const userClient = createClient(UserService, transport);
