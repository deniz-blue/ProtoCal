import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import { answerEditButton, threadOfEditButton } from "../core/meet-edit.js";
import {
	meetFormTargetOf,
	openMeetForm,
	openPickedMeetForm,
	submitMeetForm,
} from "../core/meet-form.js";
import { MEET_FORUM_PICK } from "../core/meet-post.js";

export default defineEvent({
	type: Events.InteractionCreate,
	on: async (interaction) => {
		if (interaction.isStringSelectMenu() && interaction.customId === MEET_FORUM_PICK)
			return openPickedMeetForm(interaction);
		if (!interaction.isButton() && !interaction.isModalSubmit()) return;
		const editThread = threadOfEditButton(interaction.customId);
		if (editThread && interaction.isButton()) return answerEditButton(interaction, editThread);
		const target = meetFormTargetOf(interaction.customId);
		if (!target) return;
		if (interaction.isButton()) await openMeetForm(interaction, target);
		else await submitMeetForm(interaction, target);
	},
});
