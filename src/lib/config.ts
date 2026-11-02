export const HTTP_HOST = process.env.PROTOCAL_HTTP_HOST ?? "127.0.0.1";

export const HTTP_PORT = Number(process.env.PROTOCAL_HTTP_PORT ?? 8660);

export const PUBLIC_URL = (
	process.env.PROTOCAL_PUBLIC_URL ?? `http://localhost:${HTTP_PORT}`
).replace(/\/$/, "");

export const EDITOR_URL = process.env.PROTOCAL_EDITOR_URL ?? "https://eventsl.ink/edit";

export const VIEWER_URL = process.env.PROTOCAL_VIEWER_URL ?? "https://eventsl.ink/event";

export const meetDocumentUrl = (threadId: string) => `${PUBLIC_URL}/events/${threadId}.evnt.json`;

export const VANTAGE_URL = (process.env.PROTOCAL_VANTAGE_URL ?? "https://vantage.tsx.lt").replace(
	/\/$/,
	"",
);

export const forumIndexUrl = (forumId: string) => `${PUBLIC_URL}/forums/${forumId}.json`;

export const calendarIndexUrl = (token: string) => `${PUBLIC_URL}/calendars/${token}.json`;

export const calendarDocumentUrl = (token: string, threadId: string) =>
	`${PUBLIC_URL}/calendars/${token}/events/${threadId}.evnt.json`;

export const vantageSubscribeUrl = (indexUrl: string) =>
	`${VANTAGE_URL}/settings/calendars?subscribe=${encodeURIComponent(indexUrl)}`;
