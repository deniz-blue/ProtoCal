import { Hono, type Context } from "hono";
import { cors } from "hono/cors";
import { calendarLinkOfToken } from "../db/calendar-links.js";
import { meetForumOf, meetOfThread, meetsOfForum } from "../db/meets.js";
import type { Meet } from "../db/schema.js";
import { EDITOR_URL, PUBLIC_URL, calendarDocumentUrl, meetDocumentUrl } from "../lib/config.js";
import { meetsJoinedBy } from "../core/calendar.js";
import { etagOf, meetDocumentBody, openEvntIndexOf } from "../core/meet-index.js";
import { meetEditGrants } from "../core/meet-edit.js";
import { textOf, validateMeetDocument } from "../core/meet-document.js";
import { saveMeetDocument } from "../core/meet.js";
import { everyoneCanViewForum } from "../core/meet-access.js";
import { client } from "../services/client.js";
import {
	announceableRolesOf,
	forumConfigsOf,
	guildOfConfigGrant,
	reviewChannelsOf,
	saveGuildConfig,
} from "../core/guild-config.js";
import { configPage, guildConfigChangeOfForm } from "./config-page.js";
import { editReturnPage } from "./edit-return-page.js";

const OPEN_EVNT_MEDIA_TYPE = "application/vnd.evnt+json";

const answerUnlessUnchanged = (context: Context, body: string, mediaType: string) => {
	const etag = etagOf(body);
	context.header("ETag", etag);
	context.header("Cache-Control", "no-cache");
	const known =
		context.req
			.header("If-None-Match")
			?.split(",")
			.map((tag) => tag.trim()) ?? [];
	if (known.includes(etag) || known.includes("*")) return context.body(null, 304);
	context.header("Content-Type", mediaType);
	return context.body(body);
};

const answerMeetDocument = (context: Context, meet: Meet) =>
	answerUnlessUnchanged(context, meetDocumentBody(meet), OPEN_EVNT_MEDIA_TYPE);

const answerIndex = (context: Context, meets: Meet[], hrefOf: (meet: Meet) => string) =>
	answerUnlessUnchanged(
		context,
		JSON.stringify(
			openEvntIndexOf(
				meets.map((meet) => ({ meet, href: hrefOf(meet), name: textOf(meet.document.name) })),
			),
		),
		"application/json; charset=utf-8",
	);

const everyoneCanView = async (meet: Meet) => {
	const guild = client.guilds.cache.get(meet.guildId);
	return guild ? everyoneCanViewForum(guild, meet.forumId) : false;
};

const CONFIG_LINK_EXPIRED =
	"This settings link has run out, or you can no longer manage this server. Run /config again.";

type EditReturn = { state?: unknown; cancelled?: unknown; event?: unknown };

const parsedEvent = (event: unknown) => {
	if (typeof event !== "string") return event;
	try {
		return JSON.parse(event) as unknown;
	} catch {
		return null;
	}
};

const nameOfFile = (file: string) => file.split(".")[0]!;

export const app = new Hono();

const openEvntCors = cors({
	origin: "*",
	allowMethods: ["GET"],
	allowHeaders: ["If-None-Match"],
	exposeHeaders: ["ETag"],
});
app.use("/events/*", openEvntCors);
app.use("/forums/*", openEvntCors);
app.use("/calendars/*", openEvntCors);

app.get("/events/:file{[0-9]+\\.evnt\\.json}", async (context) => {
	const meet = await meetOfThread(nameOfFile(context.req.param("file")));
	if (!meet || !(await everyoneCanView(meet))) return context.notFound();
	return answerMeetDocument(context, meet);
});

app.get("/forums/:file{[0-9]+\\.json}", async (context) => {
	const forum = await meetForumOf(nameOfFile(context.req.param("file")));
	const guild = forum && client.guilds.cache.get(forum.guildId);
	if (!forum || !guild || !(await everyoneCanViewForum(guild, forum.forumId)))
		return context.notFound();
	return answerIndex(context, await meetsOfForum(forum.forumId), (meet) =>
		meetDocumentUrl(meet.threadId),
	);
});

app.get("/calendars/:file{[A-Za-z0-9_-]+\\.json}", async (context) => {
	const link = await calendarLinkOfToken(nameOfFile(context.req.param("file")));
	if (!link) return context.notFound();
	return answerIndex(context, await meetsJoinedBy(link.userId), (meet) =>
		calendarDocumentUrl(link.token, meet.threadId),
	);
});

app.get("/calendars/:token{[A-Za-z0-9_-]+}/events/:file{[0-9]+\\.evnt\\.json}", async (context) => {
	const link = await calendarLinkOfToken(context.req.param("token"));
	const threadId = nameOfFile(context.req.param("file"));
	const meet =
		link && (await meetsJoinedBy(link.userId)).find((joined) => joined.threadId === threadId);
	if (!meet) return context.notFound();
	return answerMeetDocument(context, meet);
});

app.get("/edit/return", (context) => context.html(editReturnPage));

app.post("/edit/return", async (context) => {
	const answer = (status: 200 | 400 | 403, message: string, threadUrl?: string) =>
		context.json({ message, threadUrl }, status);
	const sent = (await context.req.json().catch(() => ({}))) as EditReturn;
	const token = typeof sent.state === "string" ? sent.state : "";
	const grant = meetEditGrants.find(token);
	if (!grant) return answer(403, "This edit link has run out. Ask for a new one in Discord.");
	const meet = await meetOfThread(grant.threadId);
	const thread = await client.channels.fetch(grant.threadId).catch(() => null);
	if (!meet || !thread?.isThread()) return answer(400, "That meet is over or its post is gone.");
	const threadUrl = thread.url;
	if (sent.cancelled === true) return answer(200, "Nothing was changed.", threadUrl);
	const event = parsedEvent(sent.event);
	if (
		typeof event !== "object" ||
		event === null ||
		(event as { id?: unknown }).id !== meet.document.id
	)
		return answer(400, "That is not this meet's event. Its id does not match.", threadUrl);
	const checked = validateMeetDocument(event);
	if ("problem" in checked) return answer(400, checked.problem.message, threadUrl);
	await saveMeetDocument(thread, meet, checked.document);
	meetEditGrants.spend(token);
	return answer(200, "Saved.", threadUrl);
});

app.get("/edit/:token", async (context) => {
	const token = context.req.param("token");
	const grant = meetEditGrants.find(token);
	const meet = grant && (await meetOfThread(grant.threadId));
	if (!meet) return context.text("This edit link has run out. Ask for a new one in Discord.", 403);
	const editor = new URL(EDITOR_URL);
	editor.searchParams.set("data", JSON.stringify(meet.document));
	editor.searchParams.set("return", `${PUBLIC_URL}/edit/return`);
	editor.searchParams.set("state", token);
	return context.redirect(editor.toString());
});

app.get("/config/:token", async (context) => {
	const guild = await guildOfConfigGrant(context.req.param("token"));
	if (!guild) return context.text(CONFIG_LINK_EXPIRED, 403);
	return context.html(
		configPage({
			guildName: guild.name,
			forums: await forumConfigsOf(guild),
			roles: await announceableRolesOf(guild),
			reviewChannels: await reviewChannelsOf(guild),
			saved: context.req.query("saved") !== undefined,
		}),
	);
});

app.post("/config/:token", async (context) => {
	const guild = await guildOfConfigGrant(context.req.param("token"));
	if (!guild) return context.text(CONFIG_LINK_EXPIRED, 403);
	await saveGuildConfig(guild, guildConfigChangeOfForm(await context.req.parseBody({ all: true })));
	return context.redirect(`${context.req.path}?saved`, 303);
});
