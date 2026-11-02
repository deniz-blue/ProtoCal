import { REST, Routes, type RESTPutAPIApplicationCommandsJSONBody } from "discord.js";
import { commands } from "./services/commands.js";
import { createChildLogger } from "./lib/logger.js";

const log = createChildLogger("publish");

const api = new REST().setToken(process.env.DISCORD_BOT_TOKEN!);

const main = async () => {
	log.info({ commands: commands.map((cmd) => cmd.name) }, "Publishing commands");

	const data: RESTPutAPIApplicationCommandsJSONBody = [];

	for (const command of commands) {
		data.push(command.data.toJSON());
	}

	const global = process.argv.includes("--global");

	await api.put(
		global
			? Routes.applicationCommands(process.env.DISCORD_APP_ID!)
			: Routes.applicationGuildCommands(process.env.DISCORD_APP_ID!, process.env.DISCORD_GUILD_ID!),
		{ body: data },
	);

	log.info("Commands published");
};

main()
	.catch((error) => {
		log.error(error, "Failed to publish commands");
		process.exit(1);
	})
	.finally(() => process.exit());
