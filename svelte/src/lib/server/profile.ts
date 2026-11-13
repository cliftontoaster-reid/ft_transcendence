import { AgeBracket, type User } from '$lib/proto/dionysus/user/v1/user_pb';
import type { Profile } from '$lib/profile';
import { bytesToUuid } from '$lib/uuid';

export function toProfile(user: User): Profile {
	return {
		id: bytesToUuid(user.id?.value?.value ?? new Uint8Array()),
		username: user.username,
		displayName: user.displayName,
		bio: user.bio,
		avatarUrl: user.avatarUrl,
		age: user.age ?? AgeBracket.UNSPECIFIED
	};
}
