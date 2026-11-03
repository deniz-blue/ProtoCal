import {
	ActionRowBuilder,
	MessageFlags,
	PermissionFlagsBits,
	channelMention,
	StringSelectMenuBuilder,
	type ButtonInteraction,
	type ChatInputCommandInteraction,
	type ModalSubmitInteraction,
	type StringSelectMenuInteraction,
} from "discord.js";
import { meetOfThread } from "../db/meets.js";
import { meetForumOpenTo, meetForumsOpenTo, meetGoesToReview } from "./meet-access.js";
import { postNewMeet, saveMeetDocument } from "./meet.js";
import { MEET_FORM_BUTTON, MEET_FORUM_PICK, meetFormTargetOfNew } from "./meet-post.js";
import {
	fieldsOfMeetDocument,
	meetDocumentOfFields,
	newMeetDocument,
	type MeetFields,
} from "./meet-document.js";
import {
	fieldsOfSubmittedMeetForm,
	forgetRejectedDraft,
	meetFormModal,
	rejectDraft,
	rejectedDraftOf,
} from "./meet-form-modal.js";
import { meetRequestToEdit, saveMeetRequestEdit, sendMeetRequest } from "./meet-request.js";

export const meetFormTargetOf = (customId: string) =>
	customId.startsWith(`${MEET_FORM_BUTTON}:`) ? customId.slice(MEET_FORM_BUTTON.length + 1) : null;

export const mayEditMeet = (
	interaction: ButtonInteraction | ModalSubmitInteraction,
	organizers: string[],
) =>
	organizers.includes(interaction.user.id) ||
	(interaction.memberPermissions?.has(PermissionFlagsBits.ManageThreads) ?? false);

const NO_FORUM = "There is no meet forum here that you can post in.";

const REVIEWED_HINT = "The moderators check it before it is posted";

export const offerNewMeetForm = async (interaction: ChatInputCommandInteraction) => {
	const member = await interaction.guild!.members.fetch(interaction.user.id);
	const forums = await meetForumsOpenTo(member);
	const picked = interaction.options.getChannel("forum");
	const reply = (content: string, components: ActionRowBuilder<StringSelectMenuBuilder>[] = []) =>
		interaction.reply({ content, components, flags: MessageFlags.Ephemeral, allowedMentions: {} });
	if (picked) {
		const forum = forums.find((candidate) => candidate.channel.id === picked.id);
		if (forum)
			return interaction.showModal(meetFormModal(meetFormTargetOfNew(forum.channel.id), null));
		return reply(`${channelMention(picked.id)} is not a meet forum you can post in.`);
	}
	if (forums.length === 0) return reply(NO_FORUM);
	if (forums.length === 1)
		return interaction.showModal(meetFormModal(meetFormTargetOfNew(forums[0]!.channel.id), null));
	return reply("Where should it go?", [
		new ActionRowBuilder<StringSelectMenuBuilder>().addComponents(
			new StringSelectMenuBuilder()
				.setCustomId(MEET_FORUM_PICK)
				.setPlaceholder("Pick a forum")
				.addOptions(
					forums.slice(0, 25).map((forum) => ({
						label: forum.channel.name,
						value: forum.channel.id,
						...(meetGoesToReview(member, forum) ? { description: REVIEWED_HINT } : {}),
					})),
				),
		),
	]);
};

const meetForumOpenToUser = async (
	interaction: StringSelectMenuInteraction | ModalSubmitInteraction,
	forumId: string,
) => {
	const member = await interaction.guild?.members.fetch(interaction.user.id);
	const forum = member && (await meetForumOpenTo(member, forumId));
	return member && forum ? { member, forum } : null;
};

export const openPickedMeetForm = async (interaction: StringSelectMenuInteraction) => {
	const forumId = interaction.values[0] ?? "";
	if (!(await meetForumOpenToUser(interaction, forumId))) {
		await interaction.reply({ content: NO_FORUM, flags: MessageFlags.Ephemeral });
		return;
	}
	await interaction.showModal(meetFormModal(meetFormTargetOfNew(forumId), null));
};

const ONLY_HOST_OR_STAFF = "Only the host or staff can edit this meet.";

export const openMeetForm = async (interaction: ButtonInteraction, target: string) => {
	const rejected = rejectedDraftOf(interaction.user.id, target);
	if (target.startsWith("new:")) return interaction.showModal(meetFormModal(target, rejected));

	if (target.startsWith("request:")) {
		const request = await meetRequestToEdit(interaction, target.slice("request:".length));
		if (request)
			await interaction.showModal(
				meetFormModal(target, rejected ?? fieldsOfMeetDocument(request.document)),
			);
		return;
	}

	const meet = await meetOfThread(target.slice("edit:".length));
	if (!meet || !mayEditMeet(interaction, meet.organizers)) {
		await interaction.reply({ content: ONLY_HOST_OR_STAFF, flags: MessageFlags.Ephemeral });
		return;
	}
	await interaction.showModal(
		meetFormModal(target, rejected ?? fieldsOfMeetDocument(meet.document)),
	);
};

const submitNewMeet = async (
	interaction: ModalSubmitInteraction,
	target: string,
	fields: MeetFields,
) => {
	const checked = meetDocumentOfFields(fields, newMeetDocument("urn:uuid:draft", fields.name));
	if ("problem" in checked)
		return rejectDraft(interaction, target, fields, checked.problem.message);
	await interaction.deferReply({ flags: MessageFlags.Ephemeral });
	const open = await meetForumOpenToUser(interaction, target.slice("new:".length));
	if (!open) {
		await interaction.editReply("That is not a meet forum you can post in any more.");
		return;
	}
	if (meetGoesToReview(open.member, open.forum))
		return sendMeetRequest(interaction, open.forum, checked.document, { target, fields });
	forgetRejectedDraft(interaction.user.id, target);
	const thread = await postNewMeet(open.forum.channel, [interaction.user.id], checked.document);
	await interaction.editReply(`Posted: ${channelMention(thread.id)}`);
};

export const submitMeetForm = async (interaction: ModalSubmitInteraction, target: string) => {
	const fields = fieldsOfSubmittedMeetForm(interaction);
	if (target.startsWith("new:")) return submitNewMeet(interaction, target, fields);
	if (target.startsWith("request:"))
		return saveMeetRequestEdit(interaction, target.slice("request:".length), { target, fields });

	const threadId = target.slice("edit:".length);
	const meet = await meetOfThread(threadId);
	if (!meet || !mayEditMeet(interaction, meet.organizers)) {
		await interaction.reply({ content: ONLY_HOST_OR_STAFF, flags: MessageFlags.Ephemeral });
		return;
	}
	const checked = meetDocumentOfFields(fields, meet.document);
	if ("problem" in checked)
		return rejectDraft(interaction, target, fields, checked.problem.message);
	forgetRejectedDraft(interaction.user.id, target);
	await interaction.deferReply({ flags: MessageFlags.Ephemeral });
	const thread = await interaction.guild?.channels.fetch(threadId);
	if (!thread?.isThread()) {
		await interaction.editReply("That meet's post is gone.");
		return;
	}
	await saveMeetDocument(thread, meet, checked.document);
	await interaction.editReply("Saved.");
};
