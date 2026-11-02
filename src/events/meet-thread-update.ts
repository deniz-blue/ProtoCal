import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import { endMeet, renameMeetOfThread } from "../core/meet.js";

export default defineEvent({
	type: Events.ThreadUpdate,
	on: async (before, after) => {
		if (!before.locked && after.locked) return endMeet(after.guild, after.id);
		if (before.name !== after.name) await renameMeetOfThread(after);
	},
});
