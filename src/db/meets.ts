import type { OpenEvnt } from "@evnt/schema";
import { eq } from "drizzle-orm";
import { getDb } from "./index.js";
import { meetForums, meets, type Meet, type MeetForum } from "./schema.js";

export const meetForumOf = async (forumId: string): Promise<MeetForum | null> => {
	const [forum] = await getDb().select().from(meetForums).where(eq(meetForums.forumId, forumId));
	return forum ?? null;
};

export const meetForumsOfGuild = (guildId: string): Promise<MeetForum[]> =>
	getDb().select().from(meetForums).where(eq(meetForums.guildId, guildId));

export const saveMeetForum = async (forum: MeetForum) => {
	await getDb()
		.insert(meetForums)
		.values(forum)
		.onConflictDoUpdate({ target: meetForums.forumId, set: forum });
};

export const forgetMeetForum = async (forumId: string) => {
	const forgotten = await getDb()
		.delete(meetForums)
		.where(eq(meetForums.forumId, forumId))
		.returning();
	return forgotten.length > 0;
};

export const meetOfThread = async (threadId: string): Promise<Meet | null> => {
	const [meet] = await getDb().select().from(meets).where(eq(meets.threadId, threadId));
	return meet ?? null;
};

export const saveMeet = async (meet: Meet) => {
	await getDb().insert(meets).values(meet);
};

export const replaceMeetRole = async (threadId: string, roleId: string) => {
	await getDb().update(meets).set({ roleId }).where(eq(meets.threadId, threadId));
};

export const describeMeet = async (threadId: string, document: OpenEvnt, updatedAt: number) => {
	await getDb().update(meets).set({ document, updatedAt }).where(eq(meets.threadId, threadId));
};

export const allMeets = (): Promise<Meet[]> => getDb().select().from(meets);

export const forgetMeet = async (threadId: string) => {
	await getDb().delete(meets).where(eq(meets.threadId, threadId));
};

export const meetsOfForum = (forumId: string): Promise<Meet[]> =>
	getDb().select().from(meets).where(eq(meets.forumId, forumId));
