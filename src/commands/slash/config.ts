import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	InteractionContextType,
	MessageFlags,
	PermissionFlagsBits,
	SlashCommandBuilder,
} from "discord.js";
import { defineCommand } from "../../core/define.js";
import { guildConfigGrants, mayConfigureGuild } from "../../core/guild-config.js";
import { PUBLIC_URL } from "../../lib/config.js";

export default defineCommand({
	type: "slash",
	name: "config",
	data: new SlashCommandBuilder()
		.setName("config")
		.setDescription("Open this server's ProtoCal settings")
		.setDefaultMemberPermissions(PermissionFlagsBits.ManageGuild)
		.setContexts(InteractionContextType.Guild),
	execute: async (interaction) => {
		if (!mayConfigureGuild(interaction.memberPermissions)) {
			await interaction.reply({
				content: "Only people who can manage this server can change its settings.",
				flags: MessageFlags.Ephemeral,
			});
			return;
		}
		const token = guildConfigGrants.grant({
			guildId: interaction.guildId!,
			userId: interaction.user.id,
		});
		await interaction.reply({
			content: "-# The link works for an hour, and only you have it.",
			components: [
				new ActionRowBuilder<ButtonBuilder>().addComponents(
					new ButtonBuilder()
						.setLabel("Settings")
						.setStyle(ButtonStyle.Link)
						.setURL(`${PUBLIC_URL}/config/${token}`),
				),
			],
			flags: MessageFlags.Ephemeral,
		});
	},
});
