import { randomUUID } from "node:crypto";
import {
	ActionRowBuilder,
	DiscordAPIError,
	MessageFlags,
	ModalBuilder,
	RESTJSONErrorCodes,
	TextInputBuilder,
	TextInputStyle,
	channelMention,
	userMention,
	type ButtonInteraction,
	type Client,
	type Message,
	type ModalSubmitInteraction,
} from "discord.js";
import type { OpenEvnt } from "@evnt/schema";
import {
	allMeetRequests,
	forgetMeetRequest,
	forgetUnpostedMeetRequest,
	meetRequestOf,
	moveMeetRequest,
	reopenPostingMeetRequest,
	reviseUnpostedMeetRequest,
	saveMeetRequest,
} from "../db/meet-requests.js";
import type { MeetRequest } from "../db/schema.js";
import { createChildLogger } from "../lib/logger.js";
import { forumOfGuild, isMeetModerator, type OpenMeetForum } from "./meet-access.js";
import { meetDocumentOfFields, meetIsOverAt, type MeetFields } from "./meet-document.js";
import { forgetRejectedDraft, rejectDraft } from "./meet-form-modal.js";
import { postNewMeet } from "./meet.js";
import {
	meetRequestCard,
	meetRequestCustomId,
	meetRequestHostView,
	type MeetRequestOutcome,
} from "./meet-request-view.js";

const log = createChildLogger("meet-request");

const DECLINE_REASON_MAX = 300;

const nowUnixSeconds = () => Math.floor(Date.now() / 1000);

const guildNameOf = (client: Client, request: MeetRequest) =>
	client.guilds.cache.get(request.guildId)?.name ?? "the server";

const isClosedDm = (error: unknown) =>
	error instanceof DiscordAPIError &&
	error.code === RESTJSONErrorCodes.CannotSendMessagesToThisUser;

export const sendMeetRequest = async (
	interaction: ModalSubmitInteraction,
	forum: OpenMeetForum,
	document: OpenEvnt,
	form: { target: string; fields: MeetFields },
) => {
	const id = randomUUID();
	const reviewChannel = await interaction.client.channels
		.fetch(forum.settings.reviewChannelId ?? "")
		.catch(() => null);
	if (!reviewChannel?.isSendable())
		return rejectDraft(
			interaction,
			form.target,
			form.fields,
			"This forum's review channel is gone, so I cannot send it to the moderators. Please tell staff.",
		);

	const request: MeetRequest = {
		id,
		guildId: forum.channel.guildId,
		forumId: forum.channel.id,
		hostId: interaction.user.id,
		document: { ...document, id: `urn:uuid:${id}` },
		state: "waiting",
		handledBy: null,
		declineReason: null,
		hostEdited: false,
		moderatorsEdited: false,
		cardChannelId: reviewChannel.id,
		cardMessageId: "",
		dmChannelId: null,
		dmMessageId: null,
		updatedAt: nowUnixSeconds(),
	};
	const open: MeetRequestOutcome = { kind: "open" };
	const guildName = forum.channel.guild.name;

	const dmChannel = await interaction.user.createDM();
	let sent = request;
	try {
		const withDm = { ...request, dmChannelId: dmChannel.id };
		const dm = await dmChannel.send(meetRequestHostView(withDm, open, guildName));
		sent = { ...withDm, dmMessageId: dm.id };
	} catch (error) {
		if (!isClosedDm(error)) throw error;
	}

	let card: Message;
	try {
		card = await reviewChannel.send(meetRequestCard(sent, open));
	} catch (error) {
		log.warn({ forum: forum.channel.id, error }, "Meet request card was not posted");
		if (sent.dmMessageId) await dmChannel.messages.delete(sent.dmMessageId).catch(() => {});
		return rejectDraft(
			interaction,
			form.target,
			form.fields,
			"I could not post in this forum's review channel. Please tell staff.",
		);
	}
	await saveMeetRequest({ ...sent, cardMessageId: card.id });
	forgetRejectedDraft(interaction.user.id, form.target);
	log.info({ request: id, forum: forum.channel.id }, "Meet request sent");

	await interaction.editReply(
		sent.dmChannelId
			? `Sent to the moderators of ${channelMention(forum.channel.id)}.`
			: meetRequestHostView(sent, open, guildName),
	);
};

const editCard = async (client: Client, request: MeetRequest, outcome: MeetRequestOutcome) => {
	const channel = await client.channels.fetch(request.cardChannelId).catch(() => null);
	if (!channel?.isTextBased()) return;
	await channel.messages
		.edit(request.cardMessageId, meetRequestCard(request, outcome))
		.catch((error) => log.warn({ request: request.id, error }, "Meet request card was not edited"));
};

const editHostDm = async (client: Client, request: MeetRequest, outcome: MeetRequestOutcome) => {
	if (!request.dmMessageId) return;
	const host = await client.users.fetch(request.hostId).catch(() => null);
	const dm = await host?.createDM().catch(() => null);
	await dm?.messages
		.edit(request.dmMessageId, meetRequestHostView(request, outcome, guildNameOf(client, request)))
		.catch((error) => log.warn({ request: request.id, error }, "Meet request DM was not edited"));
};

const showMeetRequest = async (
	interaction: ButtonInteraction | ModalSubmitInteraction,
	request: MeetRequest,
	outcome: MeetRequestOutcome,
) => {
	const fromCard = interaction.message?.id === request.cardMessageId;
	const fromHost = !fromCard && !!interaction.message && interaction.user.id === request.hostId;
	if (fromCard) await interaction.editReply(meetRequestCard(request, outcome));
	else await editCard(interaction.client, request, outcome);
	if (fromHost)
		await interaction.editReply(
			meetRequestHostView(request, outcome, guildNameOf(interaction.client, request)),
		);
	else await editHostDm(interaction.client, request, outcome);
};

const tell = (interaction: ButtonInteraction | ModalSubmitInteraction, content: string) =>
	interaction.deferred || interaction.replied
		? interaction.followUp({ content, flags: MessageFlags.Ephemeral, allowedMentions: {} })
		: interaction.reply({ content, flags: MessageFlags.Ephemeral, allowedMentions: {} });

const GONE = "This request is gone: it was posted, withdrawn or ran out.";

const moderatedForumOf = async (client: Client, request: MeetRequest, userId: string) => {
	const guild = client.guilds.cache.get(request.guildId);
	const forum = guild && (await forumOfGuild(guild, request.forumId));
	const member = guild && (await guild.members.fetch(userId).catch(() => null));
	return forum && member && isMeetModerator(member, forum) ? forum : null;
};

const onlyModerators = (request: MeetRequest) =>
	`Only the moderators of ${channelMention(request.forumId)} can do this.`;

const alreadyHandled = (request: MeetRequest | null) => {
	if (request?.state === "posting")
		return `${userMention(request.handledBy ?? "")} already posted it.`;
	if (request?.state === "declined")
		return `${userMention(request.handledBy ?? "")} already declined it.`;
	return GONE;
};

export const postMeetRequest = async (interaction: ButtonInteraction, id: string) => {
	await interaction.deferUpdate();
	const request = await meetRequestOf(id);
	if (!request) return tell(interaction, GONE);
	const forum = await moderatedForumOf(interaction.client, request, interaction.user.id);
	if (!forum) return tell(interaction, onlyModerators(request));
	const claimed = await moveMeetRequest(id, ["waiting", "declined"], {
		state: "posting",
		handledBy: interaction.user.id,
		declineReason: null,
	});
	if (!claimed) return tell(interaction, alreadyHandled(await meetRequestOf(id)));

	let threadId: string;
	try {
		threadId = (await postNewMeet(forum, [claimed.hostId], claimed.document)).id;
	} catch (error) {
		await reopenPostingMeetRequest(id);
		throw error;
	}
	await forgetMeetRequest(id);
	log.info({ request: id, thread: threadId, by: interaction.user.id }, "Meet request posted");
	await showMeetRequest(interaction, claimed, {
		kind: "posted",
		by: interaction.user.id,
		threadId,
	});
};

export const askDeclineReason = async (interaction: ButtonInteraction, id: string) => {
	const request = await meetRequestOf(id);
	if (!request) return tell(interaction, GONE);
	if (!(await moderatedForumOf(interaction.client, request, interaction.user.id)))
		return tell(interaction, onlyModerators(request));
	if (request.state !== "waiting") return tell(interaction, alreadyHandled(request));
	await interaction.showModal(
		new ModalBuilder()
			.setCustomId(meetRequestCustomId("decline-reason", id))
			.setTitle("Decline")
			.addComponents(
				new ActionRowBuilder<TextInputBuilder>().addComponents(
					new TextInputBuilder()
						.setCustomId("reason")
						.setLabel("Why? The host sees this")
						.setStyle(TextInputStyle.Paragraph)
						.setMaxLength(DECLINE_REASON_MAX)
						.setRequired(false),
				),
			),
	);
};

export const declineMeetRequest = async (interaction: ModalSubmitInteraction, id: string) => {
	await interaction.deferUpdate();
	const request = await meetRequestOf(id);
	if (!request) return tell(interaction, GONE);
	if (!(await moderatedForumOf(interaction.client, request, interaction.user.id)))
		return tell(interaction, onlyModerators(request));
	const declined = await moveMeetRequest(id, ["waiting"], {
		state: "declined",
		handledBy: interaction.user.id,
		declineReason: interaction.fields.getTextInputValue("reason").trim() || null,
	});
	if (!declined) return tell(interaction, alreadyHandled(await meetRequestOf(id)));
	log.info({ request: id, by: interaction.user.id }, "Meet request declined");
	await showMeetRequest(interaction, declined, { kind: "open" });
};

export const withdrawMeetRequest = async (interaction: ButtonInteraction, id: string) => {
	await interaction.deferUpdate();
	const request = await meetRequestOf(id);
	if (!request) return tell(interaction, GONE);
	if (request.hostId !== interaction.user.id)
		return tell(interaction, "Only the host can withdraw it.");
	const withdrawn = await forgetUnpostedMeetRequest(id);
	if (!withdrawn) return tell(interaction, alreadyHandled(await meetRequestOf(id)));
	log.info({ request: id }, "Meet request withdrawn");
	await showMeetRequest(interaction, withdrawn, { kind: "withdrawn" });
};

export type MeetRequestEditor = "host" | "moderator";

export const editorOfMeetRequest = async (
	client: Client,
	request: MeetRequest,
	userId: string,
): Promise<MeetRequestEditor | null> => {
	if (request.hostId === userId) return "host";
	return (await moderatedForumOf(client, request, userId)) ? "moderator" : null;
};

const ONLY_EDITORS = "Only the host or the moderators can edit this.";

export const meetRequestToEdit = async (interaction: ButtonInteraction, id: string) => {
	const request = await meetRequestOf(id);
	if (!request) {
		await tell(interaction, GONE);
		return null;
	}
	if (!(await editorOfMeetRequest(interaction.client, request, interaction.user.id))) {
		await tell(interaction, ONLY_EDITORS);
		return null;
	}
	if (request.state === "posting") {
		await tell(interaction, alreadyHandled(request));
		return null;
	}
	return request;
};

export const saveMeetRequestEdit = async (
	interaction: ModalSubmitInteraction,
	id: string,
	form: { target: string; fields: MeetFields },
) => {
	const request = await meetRequestOf(id);
	if (!request) return tell(interaction, GONE);
	const editor = await editorOfMeetRequest(interaction.client, request, interaction.user.id);
	if (!editor) return tell(interaction, ONLY_EDITORS);
	const checked = meetDocumentOfFields(form.fields, request.document);
	if ("problem" in checked)
		return rejectDraft(interaction, form.target, form.fields, checked.problem.message);
	forgetRejectedDraft(interaction.user.id, form.target);
	if (interaction.isFromMessage()) await interaction.deferUpdate();
	else await interaction.deferReply({ flags: MessageFlags.Ephemeral });

	const sendsAgain = editor === "host" && request.state === "declined";
	const edited = await reviseUnpostedMeetRequest(id, {
		document: checked.document,
		...(editor === "host" ? { hostEdited: true } : { moderatorsEdited: true }),
		...(sendsAgain ? { state: "waiting", handledBy: null, declineReason: null } : {}),
		updatedAt: nowUnixSeconds(),
	});
	if (!edited) return tell(interaction, alreadyHandled(await meetRequestOf(id)));
	log.info({ request: id, editor, sendsAgain }, "Meet request edited");
	await showMeetRequest(interaction, edited, { kind: "open" });
	if (!interaction.isFromMessage()) await interaction.editReply("Saved.");
};

export const dropMeetRequestsThatAreOver = async (client: Client, nowUnixSeconds: number) => {
	for (const request of await allMeetRequests()) {
		if (request.state === "posting") continue;
		const overAt = meetIsOverAt(request.document);
		if (overAt === null || overAt > nowUnixSeconds) continue;
		const dropped = await forgetUnpostedMeetRequest(request.id);
		if (!dropped) continue;
		log.info({ request: request.id }, "Meet request ran out");
		await editCard(client, dropped, { kind: "expired" });
		await editHostDm(client, dropped, { kind: "expired" });
	}
};
