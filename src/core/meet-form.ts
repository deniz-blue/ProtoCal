import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	MessageFlags,
	ModalBuilder,
	PermissionFlagsBits,
	TextInputBuilder,
	TextInputStyle,
	channelMention,
	StringSelectMenuBuilder,
	type ButtonInteraction,
	type ChatInputCommandInteraction,
	type ModalSubmitInteraction,
	type StringSelectMenuInteraction,
} from "discord.js";
import { meetForumOf, meetOfThread } from "../db/meets.js";
import {
	everyoneCanViewForum,
	forumOfGuild,
	meetForumsMemberCanPostIn,
	memberCanPostIn,
} from "./meet-access.js";
import { meetDocumentUrl } from "../lib/config.js";
import { saveMeetDocument, startMeet } from "./meet.js";
import {
	MEET_FORM_BUTTON,
	MEET_FORUM_PICK,
	THREAD_NAME_MAX,
	meetFormTargetOfNew,
	meetPostButtons,
	meetPostContent,
	threadNameOf,
} from "./meet-post.js";
import {
	fieldsOfMeetDocument,
	meetDocumentOfFields,
	newMeetDocument,
	type MeetFields,
} from "./meet-document.js";

const PLACE_MAX = 200;
const DESCRIPTION_MAX = 1500;
const DATE_MAX = 40;

const rejectedDrafts = new Map<string, MeetFields>();
const draftKey = (userId: string, target: string) => `${userId}:${target}`;

const meetFormModal = (target: string, filled: MeetFields | null) => {
	const input = (
		id: keyof MeetFields,
		label: string,
		maxLength: number,
		options: { required?: boolean; placeholder?: string; paragraph?: boolean } = {},
	) => {
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
			input("name", "Name", THREAD_NAME_MAX, { required: true }),
			input("start", "Start", DATE_MAX, { placeholder: "2026-12-05 18:00, or 2026-12-05" }),
			input("end", "End", DATE_MAX, { placeholder: "2026-12-06 02:00, or 2026-12-07" }),
			input("place", "Place", PLACE_MAX),
			input("description", "Description", DESCRIPTION_MAX, { paragraph: true }),
		);
};

export const meetFormTargetOf = (customId: string) =>
	customId.startsWith(`${MEET_FORM_BUTTON}:`) ? customId.slice(MEET_FORM_BUTTON.length + 1) : null;

export const mayEditMeet = (
	interaction: ButtonInteraction | ModalSubmitInteraction,
	organizers: string[],
) =>
	organizers.includes(interaction.user.id) ||
	(interaction.memberPermissions?.has(PermissionFlagsBits.ManageThreads) ?? false);

const NO_FORUM = "There is no meet forum here that you can post in.";

export const offerNewMeetForm = async (interaction: ChatInputCommandInteraction) => {
	const member = await interaction.guild!.members.fetch(interaction.user.id);
	const forums = await meetForumsMemberCanPostIn(member);
	const picked = interaction.options.getChannel("forum");
	const reply = (content: string, components: ActionRowBuilder<StringSelectMenuBuilder>[] = []) =>
		interaction.reply({ content, components, flags: MessageFlags.Ephemeral, allowedMentions: {} });
	if (picked) {
		const forum = forums.find((candidate) => candidate.id === picked.id);
		if (forum) return interaction.showModal(meetFormModal(meetFormTargetOfNew(forum.id), null));
		return reply(`${channelMention(picked.id)} is not a meet forum you can post in.`);
	}
	if (forums.length === 0) return reply(NO_FORUM);
	if (forums.length === 1)
		return interaction.showModal(meetFormModal(meetFormTargetOfNew(forums[0]!.id), null));
	return reply("Where should it go?", [
		new ActionRowBuilder<StringSelectMenuBuilder>().addComponents(
			new StringSelectMenuBuilder()
				.setCustomId(MEET_FORUM_PICK)
				.setPlaceholder("Pick a forum")
				.addOptions(forums.slice(0, 25).map((forum) => ({ label: forum.name, value: forum.id }))),
		),
	]);
};

const memberCanPostInMeetForum = async (
	interaction: StringSelectMenuInteraction | ModalSubmitInteraction,
	forumId: string,
) => {
	const guild = interaction.guild;
	const forum = guild && (await meetForumOf(forumId)) && (await forumOfGuild(guild, forumId));
	if (!forum) return null;
	const member = await guild.members.fetch(interaction.user.id);
	return memberCanPostIn(member, forum) ? forum : null;
};

export const openPickedMeetForm = async (interaction: StringSelectMenuInteraction) => {
	const forumId = interaction.values[0] ?? "";
	if (!(await memberCanPostInMeetForum(interaction, forumId))) {
		await interaction.reply({ content: NO_FORUM, flags: MessageFlags.Ephemeral });
		return;
	}
	await interaction.showModal(meetFormModal(meetFormTargetOfNew(forumId), null));
};

export const openMeetForm = async (interaction: ButtonInteraction, target: string) => {
	const rejected = rejectedDrafts.get(draftKey(interaction.user.id, target));
	if (target.startsWith("new:"))
		return interaction.showModal(meetFormModal(target, rejected ?? null));

	const meet = await meetOfThread(target.slice("edit:".length));
	if (!meet || !mayEditMeet(interaction, meet.organizers)) {
		await interaction.reply({
			content: "Only the host or staff can edit this meet.",
			flags: MessageFlags.Ephemeral,
		});
		return;
	}
	await interaction.showModal(
		meetFormModal(target, rejected ?? fieldsOfMeetDocument(meet.document)),
	);
};

const rejectDraft = async (
	interaction: ModalSubmitInteraction,
	target: string,
	fields: MeetFields,
	message: string,
) => {
	rejectedDrafts.set(draftKey(interaction.user.id, target), fields);
	await interaction.reply({
		content: message,
		components: [
			new ActionRowBuilder<ButtonBuilder>().addComponents(
				new ButtonBuilder()
					.setCustomId(`${MEET_FORM_BUTTON}:${target}`)
					.setLabel("Fix it")
					.setStyle(ButtonStyle.Primary),
			),
		],
		flags: MessageFlags.Ephemeral,
		allowedMentions: {},
	});
};

export const submitMeetForm = async (interaction: ModalSubmitInteraction, target: string) => {
	const written = (id: keyof MeetFields) => interaction.fields.getTextInputValue(id).trim() || null;
	const fields: MeetFields = {
		name: written("name") ?? "",
		start: written("start"),
		end: written("end"),
		place: written("place"),
		description: written("description"),
	};

	if (target.startsWith("new:")) {
		const forumId = target.slice("new:".length);
		const checked = meetDocumentOfFields(fields, newMeetDocument("urn:uuid:draft", fields.name));
		if ("problem" in checked)
			return rejectDraft(interaction, target, fields, checked.problem.message);
		rejectedDrafts.delete(draftKey(interaction.user.id, target));
		await interaction.deferReply({ flags: MessageFlags.Ephemeral });
		const forum = await memberCanPostInMeetForum(interaction, forumId);
		if (!forum) {
			await interaction.editReply("That is not a meet forum you can post in any more.");
			return;
		}
		const organizers = [interaction.user.id];
		const thread = await forum.threads.create({
			name: threadNameOf(checked.document),
			message: {
				content: meetPostContent(checked.document, organizers, Math.floor(Date.now() / 1000)),
				allowedMentions: {},
			},
		});
		const document = { ...checked.document, id: meetDocumentUrl(thread.id) };
		const starter = await thread.fetchStarterMessage();
		await starter?.edit({
			components: [meetPostButtons(thread.id, await everyoneCanViewForum(thread.guild, forum.id))],
		});
		await startMeet(thread, { organizers, document });
		await interaction.editReply(`Posted: ${channelMention(thread.id)}`);
		return;
	}

	const threadId = target.slice("edit:".length);
	const meet = await meetOfThread(threadId);
	if (!meet || !mayEditMeet(interaction, meet.organizers)) {
		await interaction.reply({
			content: "Only the host or staff can edit this meet.",
			flags: MessageFlags.Ephemeral,
		});
		return;
	}
	const checked = meetDocumentOfFields(fields, meet.document);
	if ("problem" in checked)
		return rejectDraft(interaction, target, fields, checked.problem.message);
	rejectedDrafts.delete(draftKey(interaction.user.id, target));
	await interaction.deferReply({ flags: MessageFlags.Ephemeral });
	const thread = await interaction.guild?.channels.fetch(threadId);
	if (!thread?.isThread()) {
		await interaction.editReply("That meet's post is gone.");
		return;
	}
	await saveMeetDocument(thread, meet, checked.document);
	await interaction.editReply("Saved.");
};
