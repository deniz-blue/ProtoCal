import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import { endMeet } from "../core/meet.js";

export default defineEvent({
	type: Events.ThreadDelete,
	on: async (thread) => endMeet(thread.guild, thread.id),
});
