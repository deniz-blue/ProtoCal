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
import { calendarIndexUrl, vantageSubscribeUrl } from "../../lib/config.js";

export default defineCommand({
	type: "slash",
	name: "mycalendar",
	data: new SlashCommandBuilder()
		.setName("mycalendar")
		.setDescription("Get a calendar link listing the meets you joined")
		.addBooleanOption((option) =>
			option.setName("new").setDescription("End your old link and make a new one"),
		)
		.setContexts(InteractionContextType.Guild, InteractionContextType.BotDM) as SlashCommandBuilder,
	execute: async (interaction) => {
		const replace = interaction.options.getBoolean("new") ?? false;
		const link = await calendarLinkOf(interaction.user.id, { replace });
		const indexUrl = calendarIndexUrl(link.token);
		await interaction.reply({
			content: [
				replace ? "Made you a new calendar link. The old one no longer works." : null,
				"It lists the meets you joined, in forums you can see, and keeps up as you join and leave.",
				`\`${indexUrl}\``,
				"-# Anyone with the link sees the same list. `/mycalendar new:True` ends it and makes another.",
			]
				.filter((line) => line !== null)
				.join("\n"),
			components: [
				new ActionRowBuilder<ButtonBuilder>().addComponents(
					new ButtonBuilder()
						.setLabel("Add to Vantage")
						.setStyle(ButtonStyle.Link)
						.setURL(vantageSubscribeUrl(indexUrl)),
				),
			],
			flags: MessageFlags.Ephemeral,
		});
	},
});
