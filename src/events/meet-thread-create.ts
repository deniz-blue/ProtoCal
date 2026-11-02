import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import { startMeet } from "../core/meet.js";

export default defineEvent({
	type: Events.ThreadCreate,
	on: async (thread, newlyCreated) => {
		if (newlyCreated && thread.ownerId !== thread.client.user.id) await startMeet(thread);
	},
});
