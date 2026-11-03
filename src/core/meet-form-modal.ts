import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	MessageFlags,
	ModalBuilder,
	TextInputBuilder,
	TextInputStyle,
	type ModalSubmitInteraction,
} from "discord.js";
import { MEET_FORM_BUTTON, THREAD_NAME_MAX } from "./meet-post.js";
import type { MeetFields } from "./meet-document.js";

const MEET_FIELD_MAX_LENGTH: Record<keyof MeetFields, number> = {
	name: THREAD_NAME_MAX,
	start: 40,
	end: 40,
	place: 200,
	description: 1500,
};

const rejectedDrafts = new Map<string, MeetFields>();
const draftKey = (userId: string, target: string) => `${userId}:${target}`;

export const rejectedDraftOf = (userId: string, target: string) =>
	rejectedDrafts.get(draftKey(userId, target)) ?? null;

export const forgetRejectedDraft = (userId: string, target: string) =>
	rejectedDrafts.delete(draftKey(userId, target));

export const meetFormModal = (target: string, filled: MeetFields | null) => {
	const input = (
		id: keyof MeetFields,
		label: string,
		options: { required?: boolean; placeholder?: string; paragraph?: boolean } = {},
	) => {
		const maxLength = MEET_FIELD_MAX_LENGTH[id];
		const textInput = new TextInputBuilder()
			.setCustomId(id)
			.setLabel(label)
			.setMaxLength(maxLength)
			.setRequired(options.required ?? false)
			.setStyle(options.paragraph ? TextInputStyle.Paragraph : TextInputStyle.Short);
		if (options.placeholder) textInput.setPlaceholder(options.placeholder);
		const value = filled?.[id];
		if (value) textInput.setValue(value.slice(0, maxLength));
		return new ActionRowBuilder<TextInputBuilder>().addComponents(textInput);
	};
	return new ModalBuilder()
		.setCustomId(`${MEET_FORM_BUTTON}:${target}`)
		.setTitle(target.startsWith("new:") ? "New meet" : "Edit meet")
		.addComponents(
			input("name", "Name", { required: true }),
			input("start", "Start", { placeholder: "2026-12-05 18:00, or 2026-12-05" }),
			input("end", "End", { placeholder: "2026-12-06 02:00, or 2026-12-07" }),
			input("place", "Place"),
			input("description", "Description", { paragraph: true }),
		);
};

export const fieldsOfSubmittedMeetForm = (interaction: ModalSubmitInteraction): MeetFields => {
	const written = (id: keyof MeetFields) => interaction.fields.getTextInputValue(id).trim() || null;
	return {
		name: written("name") ?? "",
		start: written("start"),
		end: written("end"),
		place: written("place"),
		description: written("description"),
	};
};

export const rejectDraft = async (
	interaction: ModalSubmitInteraction,
	target: string,
	fields: MeetFields,
	message: string,
) => {
	rejectedDrafts.set(draftKey(interaction.user.id, target), fields);
	const answer = {
		content: message,
		components: [
			new ActionRowBuilder<ButtonBuilder>().addComponents(
				new ButtonBuilder()
					.setCustomId(`${MEET_FORM_BUTTON}:${target}`)
					.setLabel("Fix it")
					.setStyle(ButtonStyle.Primary),
			),
		],
		allowedMentions: {},
	};
	if (interaction.deferred) await interaction.editReply(answer);
	else await interaction.reply({ ...answer, flags: MessageFlags.Ephemeral });
};
