export type ReactionEmoji = { id: string | null; name: string | null };

export const reactionKey = (emoji: ReactionEmoji): string | null => emoji.id ?? emoji.name;

export const reactionAsText = (emoji: ReactionEmoji): string =>
	emoji.id ? `<:${emoji.name ?? "emoji"}:${emoji.id}>` : (emoji.name ?? "");

// Discord refuses a role name longer than 100 characters.
const LONGEST_ROLE_NAME = 100;

export const roleNameOfMeet = (meetName: string) => `Meet: ${meetName}`.slice(0, LONGEST_ROLE_NAME);
