import {
	ChannelType,
	PermissionFlagsBits,
	type ForumChannel,
	type Guild,
	type PermissionsBitField,
	type TextChannel,
} from "discord.js";
import { allMeets, forgetMeetForum, meetForumsOfGuild, saveMeetForum } from "../db/meets.js";
import { client } from "../services/client.js";
import { createGrantStore } from "./grants.js";

export const guildConfigGrants = createGrantStore<{ guildId: string; userId: string }>();

export const mayConfigureGuild = (permissions: Readonly<PermissionsBitField> | null) =>
	permissions?.has(PermissionFlagsBits.ManageGuild) ?? false;

export const guildOfConfigGrant = async (token: string) => {
	const grant = guildConfigGrants.find(token);
	const guild = grant && client.guilds.cache.get(grant.guildId);
	if (!grant || !guild) return null;
	const member = await guild.members.fetch(grant.userId).catch(() => null);
	return member && mayConfigureGuild(member.permissions) ? guild : null;
};

export type ForumConfig = {
	forumId: string;
	name: string | null;
	isMeetForum: boolean;
	announceRoleId: string | null;
	reviewChannelId: string | null;
	hasDefaultReaction: boolean;
	everyoneCanView: boolean;
	everyoneCanPost: boolean;
};

export type GuildConfigChange = {
	meetForumIds: Set<string>;
	announceRoleIdOf: Map<string, string>;
	reviewChannelIdOf: Map<string, string>;
};

export const reviewChannelsOf = async (guild: Guild) =>
	[...(await guild.channels.fetch()).values()]
		.filter((channel): channel is TextChannel => channel?.type === ChannelType.GuildText)
		.sort((a, b) => a.rawPosition - b.rawPosition);

export const forumConfigsOf = async (guild: Guild): Promise<ForumConfig[]> => {
	const forums = [...(await guild.channels.fetch()).values()]
		.filter((channel): channel is ForumChannel => channel?.type === ChannelType.GuildForum)
		.sort((a, b) => a.rawPosition - b.rawPosition);
	const saved = new Map((await meetForumsOfGuild(guild.id)).map((forum) => [forum.forumId, forum]));
	const configs: ForumConfig[] = forums.map((forum) => {
		const everyone = forum.permissionsFor(guild.roles.everyone);
		const reviewChannelId = saved.get(forum.id)?.reviewChannelId ?? null;
		return {
			forumId: forum.id,
			name: forum.name,
			isMeetForum: saved.has(forum.id),
			announceRoleId: saved.get(forum.id)?.announceRoleId ?? null,
			reviewChannelId,
			hasDefaultReaction: !!forum.defaultReactionEmoji,
			everyoneCanView: everyone.has(PermissionFlagsBits.ViewChannel),
			everyoneCanPost: everyone.has(PermissionFlagsBits.SendMessages),
		};
	});
	for (const forum of saved.values())
		if (!forums.some(({ id }) => id === forum.forumId))
			configs.push({
				forumId: forum.forumId,
				name: null,
				isMeetForum: true,
				announceRoleId: forum.announceRoleId,
				reviewChannelId: forum.reviewChannelId,
				hasDefaultReaction: false,
				everyoneCanView: false,
				everyoneCanPost: false,
			});
	return configs;
};

export const announceableRolesOf = async (guild: Guild) => {
	const meetRoleIds = new Set(
		(await allMeets()).filter((meet) => meet.guildId === guild.id).map((meet) => meet.roleId),
	);
	return [...guild.roles.cache.values()]
		.filter((role) => role.id !== guild.id && !role.managed && !meetRoleIds.has(role.id))
		.sort((a, b) => b.position - a.position);
};

export const saveGuildConfig = async (guild: Guild, change: GuildConfigChange) => {
	const roleIds = new Set((await announceableRolesOf(guild)).map((role) => role.id));
	const reviewChannelIds = new Set((await reviewChannelsOf(guild)).map((channel) => channel.id));
	for (const forum of await forumConfigsOf(guild)) {
		if (!change.meetForumIds.has(forum.forumId)) {
			if (forum.isMeetForum) await forgetMeetForum(forum.forumId);
			continue;
		}
		const announceRoleId = change.announceRoleIdOf.get(forum.forumId) ?? "";
		const reviewChannelId = change.reviewChannelIdOf.get(forum.forumId) ?? "";
		await saveMeetForum({
			forumId: forum.forumId,
			guildId: guild.id,
			announceRoleId: roleIds.has(announceRoleId) ? announceRoleId : null,
			reviewChannelId: reviewChannelIds.has(reviewChannelId) ? reviewChannelId : null,
		});
	}
};
