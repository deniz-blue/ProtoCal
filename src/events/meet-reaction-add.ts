import { Events } from "discord.js";
import { reactionKey } from "../core/attend-reaction.js";
import { defineEvent } from "../core/define.js";
import { changeAttendance } from "../core/meet.js";

export default defineEvent({
	type: Events.MessageReactionAdd,
	on: async (reaction, user) => {
		const thread = reaction.message.channel;
		if (!thread.isThread() || reaction.message.id !== thread.id) return;
		await changeAttendance("add", thread, reactionKey(reaction.emoji), user);
	},
});
