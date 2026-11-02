import { Events, MessageFlags } from "discord.js";
import { defineEvent } from "../core/define.js";
import { djsx } from "discord-jsx-renderer";
import { commands } from "../services/commands.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("interaction");

export default defineEvent({
	type: Events.InteractionCreate,
	on: async (interaction) => {
		log.debug({ id: interaction.id, type: interaction.type }, "Interaction received");

		djsx.dispatchInteraction(interaction);

		if (interaction.isChatInputCommand() || interaction.isContextMenuCommand()) {
			const command = commands.find((cmd) => cmd.name === interaction.commandName);

			if (!command) {
				await interaction.reply({
					content: "Command not found. Please report this.",
					flags: MessageFlags.Ephemeral,
				});
				return;
			}

			await command.execute(interaction as any);
		}
	},
});
