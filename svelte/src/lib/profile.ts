import { AgeBracket } from '$lib/proto/dionysus/user/v1/user_pb';

/** Plain, serialisable view of the proto `User` sent to the pages. */
export interface Profile {
	id: string;
	username: string;
	displayName: string;
	bio: string;
	avatarUrl: string;
	age: AgeBracket;
}

// Generated enums are numeric: AGE_BRACKET_PEGI_3 -> AgeBracket.PEGI_3
export const ageLabels: Record<AgeBracket, string> = {
	[AgeBracket.UNSPECIFIED]: 'Unspecified',
	[AgeBracket.PEGI_3]: 'PEGI 3',
	[AgeBracket.PEGI_7]: 'PEGI 7',
	[AgeBracket.PEGI_12]: 'PEGI 12',
	[AgeBracket.PEGI_16]: 'PEGI 16',
	[AgeBracket.PEGI_18]: 'PEGI 18'
};

export const ageBrackets: AgeBracket[] = [
	AgeBracket.UNSPECIFIED,
	AgeBracket.PEGI_3,
	AgeBracket.PEGI_7,
	AgeBracket.PEGI_12,
	AgeBracket.PEGI_16,
	AgeBracket.PEGI_18
];
