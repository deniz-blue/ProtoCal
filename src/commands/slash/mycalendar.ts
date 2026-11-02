import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	InteractionContextType,
	MessageFlags,
	SlashCommandBuilder,
} from "discord.js";
import { defineCommand } from "../../core/define.js";
import { calendarLinkOf } from "../../core/calendar.js";
import { calendarIndexUrl, indexViewUrl } from "../../lib/config.js";

export default defineCommand({
	type: "slash",
	name: "mycalendar",
	data: new SlashCommandBuilder()
		.setName("mycalendar")
		.setDescription("Link to the meets you joined")
		.addBooleanOption((option) => option.setName("new").setDescription("Replace your link"))
		.setContexts(InteractionContextType.Guild, InteractionContextType.BotDM) as SlashCommandBuilder,
	execute: async (interaction) => {
		const replace = interaction.options.getBoolean("new") ?? false;
		const link = await calendarLinkOf(interaction.user.id, { replace });
		const indexUrl = calendarIndexUrl(link.token);
		await interaction.reply({
			components: [
				new ActionRowBuilder<ButtonBuilder>().addComponents(
					new ButtonBuilder()
						.setLabel("Open")
						.setStyle(ButtonStyle.Link)
						.setURL(indexViewUrl(indexUrl)),
				),
			],
			flags: MessageFlags.Ephemeral,
		});
	},
});
