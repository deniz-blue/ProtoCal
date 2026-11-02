import { randomBytes } from "node:crypto";
import { PermissionFlagsBits } from "discord.js";
import { calendarLinkOfUser, saveCalendarLink } from "../db/calendar-links.js";
import { allMeets } from "../db/meets.js";
import type { Meet } from "../db/schema.js";
import { client } from "../services/client.js";
import { forumOfGuild } from "./meet-access.js";

export const calendarLinkOf = async (userId: string, { replace }: { replace: boolean }) => {
	const existing = await calendarLinkOfUser(userId);
	if (existing && !replace) return existing;
	const link = {
		userId,
		token: randomBytes(24).toString("base64url"),
		createdAt: Math.floor(Date.now() / 1000),
	};
	await saveCalendarLink(link);
	return link;
};

export const meetsJoinedBy = async (userId: string) => {
	const joined: Meet[] = [];
	for (const [guildId, meets] of Map.groupBy(await allMeets(), (meet) => meet.guildId)) {
		const guild = client.guilds.cache.get(guildId);
		const member =
			guild && (await guild.members.fetch({ user: userId, force: true }).catch(() => null));
		if (!guild || !member) continue;
		for (const meet of meets) {
			if (!member.roles.cache.has(meet.roleId)) continue;
			const forum = await forumOfGuild(guild, meet.forumId);
			if (forum?.permissionsFor(member).has(PermissionFlagsBits.ViewChannel)) joined.push(meet);
		}
	}
	return joined;
};
