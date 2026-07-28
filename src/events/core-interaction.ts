import { Events, MessageFlags } from "discord.js";
import { defineEvent } from "../core/define.js";
import { djsx } from "discord-jsx-renderer";
import { commands } from "../services/commands.js";

export default defineEvent({
	type: Events.InteractionCreate,
	on: async (interaction) => {
		console.log("Interaction created:", interaction.id, interaction.type);

		djsx.dispatchInteraction(interaction);
		if (interaction.isChatInputCommand() || interaction.isContextMenuCommand()) {
			const command = commands.find((cmd) => cmd.name === interaction.commandName);

			if (!command)
				return void (await interaction.reply({
					content: "Command not found. Please report this.",
					flags: MessageFlags.Ephemeral,
				}));

			try {
				await command.execute(interaction as any);
			} catch {
				// if (obj instanceof ErrClass) {
				// 	logger.warn(obj, "Replying with custom error message");
				// 	if (interaction.replied || interaction.deferred) {
				// 		interaction.followUp({
				// 			content: obj.payload,
				// 			flags: [MessageFlags.Ephemeral],
				// 		});
				// 	} else {
				// 		interaction.reply({
				// 			content: obj.payload,
				// 			flags: [MessageFlags.Ephemeral],
				// 		});
				// 	}
				// } else {
				// 	logger.error(obj, "Error executing command");
				// 	interaction.reply({
				// 		content: `Error! Please report: ${obj}`,
				// 	});
				// }
			}
		}
	},
});
