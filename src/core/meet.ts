import {
	ChannelType,
	DiscordAPIError,
	RESTJSONErrorCodes,
	roleMention,
	userMention,
	type AnyThreadChannel,
	type Client,
	type Guild,
	type PartialUser,
	type User,
} from "discord.js";
import {
	forgetMeet,
	meetForumOf,
	meetOfThread,
	allMeets,
	describeMeet,
	replaceMeetRole,
	saveMeet,
} from "../db/meets.js";
import type { OpenEvnt } from "@evnt/schema";
import type { Meet } from "../db/schema.js";
import { meetDocumentUrl } from "../lib/config.js";
import { meetChangeNotice, meetIsOverAt, newMeetDocument } from "./meet-document.js";
import { meetPostButtons, meetPostContent, threadNameOf } from "./meet-post.js";
import { everyoneCanViewForum } from "./meet-access.js";
import { createChildLogger } from "../lib/logger.js";
import { reactionAsText, reactionKey, roleNameOfMeet } from "./attend-reaction.js";

const log = createChildLogger("meet");

export type MeetAsPosted = Pick<Meet, "organizers" | "document">;

const nowUnixSeconds = () => Math.floor(Date.now() / 1000);

export type MeetStart = "started" | "not-a-meet-forum" | "no-attend-reaction";

export const startMeet = async (
	thread: AnyThreadChannel,
	posted?: MeetAsPosted,
): Promise<MeetStart> => {
	if (thread.parent?.type !== ChannelType.GuildForum) return "not-a-meet-forum";
	const forum = await meetForumOf(thread.parent.id);
	if (!forum) return "not-a-meet-forum";

	const attendEmoji = thread.parent.defaultReactionEmoji;
	const attendReaction = attendEmoji && reactionKey(attendEmoji);
	if (!attendEmoji || !attendReaction) {
		await thread.send(
			"-# ❌ This forum has no default reaction, so there is nothing to react with to attend. Staff can set one in the forum's settings.",
		);
		return "no-attend-reaction";
	}

	const meetAsPosted = posted ?? {
		organizers: thread.ownerId ? [thread.ownerId] : [],
		document: newMeetDocument(meetDocumentUrl(thread.id), thread.name),
	};
	const role = await createMeetRole(thread);
	await saveMeet({
		threadId: thread.id,
		guildId: thread.guild.id,
		forumId: forum.forumId,
		roleId: role.id,
		attendReaction,
		...meetAsPosted,
		updatedAt: nowUnixSeconds(),
	});
	log.info({ thread: thread.id, role: role.id }, "Meet started");

	const starter = await thread.fetchStarterMessage();
	const attendees = new Set(meetAsPosted.organizers);
	if (starter) {
		const earlierReaction = starter.reactions.cache.find(
			(reaction) => reactionKey(reaction.emoji) === attendReaction,
		);
		if (earlierReaction)
			for (const user of (await earlierReaction.users.fetch()).values())
				if (!user.bot) attendees.add(user.id);
		await starter.react(attendEmoji.id ?? attendEmoji.name!);
		await starter.pin();
	}
	for (const attendee of attendees) await changeMeetRole(thread, "add", attendee, role.id);

	const announced = forum.announceRoleId ? [forum.announceRoleId] : [];
	await thread.send({
		content: [
			...announced.map(roleMention),
			`-# Role: ${roleMention(role.id)}`,
			`-# React with ${reactionAsText(attendEmoji)} on the first message if you are looking to attend, to get the role`,
		].join("\n"),
		allowedMentions: { roles: announced },
	});
	return "started";
};

export const endMeet = async (guild: Guild, threadId: string) => {
	const meet = await meetOfThread(threadId);
	if (!meet) return;
	await guild.roles.delete(meet.roleId, `Meet ${threadId} ended`).catch((error) => {
		log.warn({ thread: threadId, error }, "Meet role was not deleted");
	});
	await forgetMeet(threadId);
	log.info({ thread: threadId }, "Meet ended");
};

export const endMeetsThatAreOver = async (client: Client<true>, nowUnixSeconds: number) => {
	for (const meet of await allMeets()) {
		const overAt = meetIsOverAt(meet.document);
		if (overAt === null || overAt > nowUnixSeconds) continue;
		const guild = client.guilds.cache.get(meet.guildId);
		if (guild) await endMeet(guild, meet.threadId);
	}
};

export const renameMeetOfThread = async (thread: AnyThreadChannel) => {
	const meet = await meetOfThread(thread.id);
	if (!meet || threadNameOf(meet.document) === thread.name) return;
	await saveMeetDocument(thread, meet, { ...meet.document, name: { und: thread.name } });
};

export const saveMeetDocument = async (
	thread: AnyThreadChannel,
	meet: Meet,
	document: OpenEvnt,
) => {
	const updatedAt = nowUnixSeconds();
	await describeMeet(thread.id, document, updatedAt);
	const starter = await thread.fetchStarterMessage().catch(() => null);
	if (starter?.author.id === thread.client.user.id)
		await starter.edit({
			content: meetPostContent(document, meet.organizers, updatedAt),
			components: [
				meetPostButtons(thread.id, await everyoneCanViewForum(thread.guild, meet.forumId)),
			],
			allowedMentions: {},
		});
	const name = threadNameOf(document);
	if (thread.name !== name) await thread.setName(name);
	if (threadNameOf(meet.document) !== name)
		await thread.guild.roles.edit(meet.roleId, { name: roleNameOfMeet(name) }).catch((error) => {
			log.warn({ thread: thread.id, error }, "Meet role was not renamed");
		});
	const notice = meetChangeNotice(meet.document, document);
	if (notice)
		await thread.send({
			content: [...(notice.pingsRole ? [roleMention(meet.roleId)] : []), ...notice.lines].join(
				"\n",
			),
			allowedMentions: { roles: notice.pingsRole ? [meet.roleId] : [] },
		});
};

export const changeAttendance = async (
	change: "add" | "remove",
	thread: AnyThreadChannel,
	reaction: string | null,
	user: User | PartialUser,
) => {
	if (user.bot) return;
	const meet = await meetOfThread(thread.id);
	if (!meet || meet.attendReaction !== reaction) return;
	await changeMeetRole(thread, change, user.id, meet.roleId);
};

const createMeetRole = (thread: AnyThreadChannel) =>
	thread.guild.roles.create({
		name: roleNameOfMeet(thread.name),
		mentionable: true,
		permissions: [],
		reason: `Meet ${thread.id}`,
	});

const isUnknownRole = (error: unknown) =>
	error instanceof DiscordAPIError && error.code === RESTJSONErrorCodes.UnknownRole;

const addMeetRole = async (thread: AnyThreadChannel, userId: string, roleId: string) => {
	try {
		await thread.guild.members.addRole({ user: userId, role: roleId });
	} catch (error) {
		if (!isUnknownRole(error)) throw error;
		const role = await createMeetRole(thread);
		await replaceMeetRole(thread.id, role.id);
		log.info({ thread: thread.id, gone: roleId, role: role.id }, "Meet role made again");
		await thread.guild.members.addRole({ user: userId, role: role.id });
	}
};

const changeMeetRole = async (
	thread: AnyThreadChannel,
	change: "add" | "remove",
	userId: string,
	roleId: string,
) => {
	try {
		if (change === "add") await addMeetRole(thread, userId, roleId);
		else await thread.guild.members.removeRole({ user: userId, role: roleId });
	} catch (error) {
		if (change === "remove" && isUnknownRole(error)) return;
		log.error({ thread: thread.id, user: userId, change, error }, "Meet role was not changed");
		await thread.send({
			content: `-# ❌ I could not ${change === "add" ? "give" : "take"} the role for ${userMention(userId)}. Please tell staff.`,
			allowedMentions: {},
		});
	}
};
