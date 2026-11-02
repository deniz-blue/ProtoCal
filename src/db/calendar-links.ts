import { eq } from "drizzle-orm";
import { getDb } from "./index.js";
import { calendarLinks, type CalendarLink } from "./schema.js";

export const calendarLinkOfUser = async (userId: string): Promise<CalendarLink | null> => {
	const [link] = await getDb().select().from(calendarLinks).where(eq(calendarLinks.userId, userId));
	return link ?? null;
};

export const calendarLinkOfToken = async (token: string): Promise<CalendarLink | null> => {
	const [link] = await getDb().select().from(calendarLinks).where(eq(calendarLinks.token, token));
	return link ?? null;
};

export const saveCalendarLink = async (link: CalendarLink) => {
	await getDb()
		.insert(calendarLinks)
		.values(link)
		.onConflictDoUpdate({ target: calendarLinks.userId, set: link });
};
