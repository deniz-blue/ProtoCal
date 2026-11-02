import { Events, type Message } from "discord.js";
import { defineEvent } from "../core/define.js";
import { meetOfThread } from "../db/meets.js";

const ONLY_ROLE_MENTIONS = /^(\s*<@&\d+>)+\s*$/;

const repliedToMessage = (message: Message) =>
	message.reference?.messageId ? message.fetchReference().catch(() => null) : null;

export default defineEvent({
	type: Events.MessageCreate,
	on: async (message) => {
		if (message.author.bot || message.mentions.roles.size === 0) return;
		if (!message.channel.isThread()) return;
		const meet = await meetOfThread(message.channel.id);
		if (!meet || !message.mentions.roles.has(meet.roleId)) return;
		const onlyThePing = ONLY_ROLE_MENTIONS.test(message.content);
		const repliedTo = await repliedToMessage(message);
		if (repliedTo && (onlyThePing || repliedTo.poll || repliedTo.author.bot)) {
			await repliedTo.pin();
			if (onlyThePing) return;
		}
		await message.pin();
	},
});
