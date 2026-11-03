import {
	ChannelType,
	PermissionFlagsBits,
	type ForumChannel,
	type Guild,
	type GuildMember,
} from "discord.js";
import { meetForumOf, meetForumsOfGuild } from "../db/meets.js";
import type { MeetForum } from "../db/schema.js";

export const forumOfGuild = async (guild: Guild, forumId: string) => {
	const forum = await guild.channels.fetch(forumId).catch(() => null);
	return forum?.type === ChannelType.GuildForum ? forum : null;
};

export const everyoneCanViewForum = async (guild: Guild, forumId: string) => {
	const forum = await forumOfGuild(guild, forumId);
	return forum?.permissionsFor(guild.roles.everyone).has(PermissionFlagsBits.ViewChannel) ?? false;
};

export const isMeetModerator = (member: GuildMember, forum: ForumChannel) =>
	forum.permissionsFor(member).has(PermissionFlagsBits.ManageThreads);

export const isScreened = (forum: MeetForum) => forum.reviewChannelId !== null;

export type OpenMeetForum = { channel: ForumChannel; settings: MeetForum };

export const meetGoesToReview = (member: GuildMember, forum: OpenMeetForum) =>
	isScreened(forum.settings) && !isMeetModerator(member, forum.channel);

const memberMayPostMeetIn = (member: GuildMember, forum: OpenMeetForum) =>
	forum.channel
		.permissionsFor(member)
		.has(
			isScreened(forum.settings)
				? [PermissionFlagsBits.ViewChannel]
				: [PermissionFlagsBits.ViewChannel, PermissionFlagsBits.SendMessages],
		);

export const meetForumsOpenTo = async (member: GuildMember) => {
	const forums: OpenMeetForum[] = [];
	for (const settings of await meetForumsOfGuild(member.guild.id)) {
		const channel = await forumOfGuild(member.guild, settings.forumId);
		if (channel && memberMayPostMeetIn(member, { channel, settings }))
			forums.push({ channel, settings });
	}
	return forums;
};

export const meetForumOpenTo = async (
	member: GuildMember,
	forumId: string,
): Promise<OpenMeetForum | null> => {
	const settings = await meetForumOf(forumId);
	const channel = settings && (await forumOfGuild(member.guild, forumId));
	if (!settings || !channel) return null;
	return memberMayPostMeetIn(member, { channel, settings }) ? { channel, settings } : null;
};
