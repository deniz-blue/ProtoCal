import { and, eq } from "drizzle-orm";
import { getDb } from "./index.js";
import { meetRequests, type MeetRequest } from "./schema.js";

type RequestMatch = { onlyIfWaiting?: boolean };

const whereRequest = (id: string, { onlyIfWaiting = false }: RequestMatch) =>
	onlyIfWaiting
		? and(eq(meetRequests.id, id), eq(meetRequests.state, "waiting"))
		: eq(meetRequests.id, id);

export const meetRequestOf = async (id: string): Promise<MeetRequest | null> => {
	const [request] = await getDb().select().from(meetRequests).where(eq(meetRequests.id, id));
	return request ?? null;
};

export const allMeetRequests = (): Promise<MeetRequest[]> => getDb().select().from(meetRequests);

export const saveMeetRequest = async (request: MeetRequest) => {
	await getDb().insert(meetRequests).values(request);
};

export const reviseMeetRequest = async (
	id: string,
	change: Partial<Omit<MeetRequest, "id">>,
	match: RequestMatch = {},
): Promise<MeetRequest | null> => {
	const [revised] = await getDb()
		.update(meetRequests)
		.set(change)
		.where(whereRequest(id, match))
		.returning();
	return revised ?? null;
};

export const forgetMeetRequest = async (
	id: string,
	match: RequestMatch = {},
): Promise<MeetRequest | null> => {
	const [forgotten] = await getDb().delete(meetRequests).where(whereRequest(id, match)).returning();
	return forgotten ?? null;
};
