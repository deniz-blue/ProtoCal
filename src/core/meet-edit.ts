import { MessageFlags, type ButtonInteraction } from "discord.js";
import { meetOfThread } from "../db/meets.js";
import { PUBLIC_URL } from "../lib/config.js";
import { createGrantStore } from "./grants.js";
import { mayEditMeet } from "./meet-form.js";
import { MEET_EDIT_BUTTON, meetEditButtons } from "./meet-post.js";

export const meetEditGrants = createGrantStore<{ threadId: string; userId: string }>();

export const threadOfEditButton = (customId: string) =>
	customId.startsWith(`${MEET_EDIT_BUTTON}:`) ? customId.slice(MEET_EDIT_BUTTON.length + 1) : null;

export const answerEditButton = async (interaction: ButtonInteraction, threadId: string) => {
	const meet = await meetOfThread(threadId);
	if (!meet || !mayEditMeet(interaction, meet.organizers)) {
		await interaction.reply({
			content: "Only the host or staff can edit this meet.",
			flags: MessageFlags.Ephemeral,
		});
		return;
	}
	const editUrl = `${PUBLIC_URL}/edit/${meetEditGrants.grant({ threadId, userId: interaction.user.id })}`;
	await interaction.reply({
		components: [meetEditButtons(threadId, editUrl)],
		flags: MessageFlags.Ephemeral,
	});
};
