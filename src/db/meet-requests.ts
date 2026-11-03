import { and, eq, inArray } from "drizzle-orm";
import { getDb } from "./index.js";
import { meetRequests, type MeetRequest } from "./schema.js";

export const meetRequestOf = async (id: string): Promise<MeetRequest | null> => {
	const [request] = await getDb().select().from(meetRequests).where(eq(meetRequests.id, id));
	return request ?? null;
};

export const allMeetRequests = (): Promise<MeetRequest[]> => getDb().select().from(meetRequests);

export const saveMeetRequest = async (request: MeetRequest) => {
	await getDb().insert(meetRequests).values(request);
};

export const reviseUnpostedMeetRequest = async (
	id: string,
	change: Partial<Omit<MeetRequest, "id">>,
): Promise<MeetRequest | null> => {
	const [revised] = await getDb()
		.update(meetRequests)
		.set(change)
		.where(and(eq(meetRequests.id, id), inArray(meetRequests.state, ["waiting", "declined"])))
		.returning();
	return revised ?? null;
};

export const moveMeetRequest = async (
	id: string,
	from: MeetRequest["state"][],
	change: Pick<MeetRequest, "state" | "handledBy" | "declineReason">,
): Promise<MeetRequest | null> => {
	const [moved] = await getDb()
		.update(meetRequests)
		.set(change)
		.where(and(eq(meetRequests.id, id), inArray(meetRequests.state, from)))
		.returning();
	return moved ?? null;
};

export const reopenPostingMeetRequest = async (id: string) => {
	await getDb()
		.update(meetRequests)
		.set({ state: "waiting", handledBy: null })
		.where(and(eq(meetRequests.id, id), eq(meetRequests.state, "posting")));
};

export const forgetMeetRequest = async (id: string) => {
	await getDb().delete(meetRequests).where(eq(meetRequests.id, id));
};

export const forgetUnpostedMeetRequest = async (id: string): Promise<MeetRequest | null> => {
	const [forgotten] = await getDb()
		.delete(meetRequests)
		.where(and(eq(meetRequests.id, id), inArray(meetRequests.state, ["waiting", "declined"])))
		.returning();
	return forgotten ?? null;
};
