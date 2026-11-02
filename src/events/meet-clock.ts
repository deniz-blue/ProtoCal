import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import { endMeetsThatAreOver } from "../core/meet.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("meet-clock");

const CHECK_EVERY_MS = 15 * 60 * 1000;

let checking: NodeJS.Timeout | undefined;

export default defineEvent({
	type: Events.ClientReady,
	on: async (client) => {
		const check = async () => {
			const nowUnixSeconds = Math.floor(Date.now() / 1000);
			await endMeetsThatAreOver(client, nowUnixSeconds);
		};
		const checkAndLog = () => check().catch((error) => log.error({ error }, "Meet check failed"));
		clearInterval(checking);
		checking = setInterval(checkAndLog, CHECK_EVERY_MS);
		await checkAndLog();
	},
});

if (import.meta.hot) import.meta.hot.dispose(() => clearInterval(checking));
