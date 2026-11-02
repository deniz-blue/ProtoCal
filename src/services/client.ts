import { Client, Partials } from "discord.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("client");

export let client: Client = new Client({
	intents: ["Guilds", "GuildMessageReactions", "GuildMessages"],
	partials: [Partials.Reaction, Partials.Message, Partials.Channel, Partials.User],
});

const { DISCORD_BOT_TOKEN } = process.env;

if (DISCORD_BOT_TOKEN) client.login(DISCORD_BOT_TOKEN);
else log.error("DISCORD_BOT_TOKEN is not set in the environment variables");

if (import.meta.hot) {
	import.meta.hot.accept((newModule) => {
		client = (newModule as any).client;
	});
}
