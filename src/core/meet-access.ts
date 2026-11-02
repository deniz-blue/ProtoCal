import {
	ChannelType,
	PermissionFlagsBits,
	type ForumChannel,
	type Guild,
	type GuildMember,
} from "discord.js";
import { meetForumsOfGuild } from "../db/meets.js";

export const forumOfGuild = async (guild: Guild, forumId: string) => {
	const forum = await guild.channels.fetch(forumId).catch(() => null);
	return forum?.type === ChannelType.GuildForum ? forum : null;
};

export const everyoneCanViewForum = async (guild: Guild, forumId: string) => {
	const forum = await forumOfGuild(guild, forumId);
	return forum?.permissionsFor(guild.roles.everyone).has(PermissionFlagsBits.ViewChannel) ?? false;
};

export const memberCanPostIn = (member: GuildMember, forum: ForumChannel) =>
	forum
		.permissionsFor(member)
		.has([PermissionFlagsBits.ViewChannel, PermissionFlagsBits.SendMessages]);

export const meetForumsMemberCanPostIn = async (member: GuildMember) => {
	const forums: ForumChannel[] = [];
	for (const { forumId } of await meetForumsOfGuild(member.guild.id)) {
		const forum = await forumOfGuild(member.guild, forumId);
		if (forum && memberCanPostIn(member, forum)) forums.push(forum);
	}
	return forums;
};
