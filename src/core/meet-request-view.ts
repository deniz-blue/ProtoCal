import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	channelMention,
	userMention,
} from "discord.js";
import type { MeetRequest } from "../db/schema.js";
import {
	MEET_FORM_BUTTON,
	MESSAGE_MAX,
	meetFormTargetOfRequest,
	meetPostContent,
} from "./meet-post.js";

const MEET_REQUEST_BUTTON = "meet-request";

export type MeetRequestAction = "post" | "decline" | "decline-reason" | "withdraw";

export const meetRequestCustomId = (action: MeetRequestAction, id: string) =>
	`${MEET_REQUEST_BUTTON}:${action}:${id}`;

export const meetRequestActionOf = (customId: string) => {
	const [prefix, action, id] = customId.split(":");
	if (prefix !== MEET_REQUEST_BUTTON || !action || !id) return null;
	return { action: action as MeetRequestAction, id };
};

export type MeetRequestOutcome =
	| { kind: "open" }
	| { kind: "posted"; by: string; threadId: string }
	| { kind: "withdrawn" }
	| { kind: "expired" };

const EXPIRED = "⌛ Nobody got to it before the meet would have been over.";

const button = (customId: string, label: string, style = ButtonStyle.Secondary) =>
	new ButtonBuilder().setCustomId(customId).setLabel(label).setStyle(style);

const editButton = (request: MeetRequest, label: string) =>
	button(`${MEET_FORM_BUTTON}:${meetFormTargetOfRequest(request.id)}`, label);

const withPreview = (request: MeetRequest, head: string[], buttons: ButtonBuilder[] = []) => {
	const top = head.filter(Boolean).join("\n");
	const preview = meetPostContent(
		request.document,
		[request.hostId],
		request.updatedAt,
		MESSAGE_MAX - top.length - 2,
	);
	return {
		content: `${top}\n\n${preview}`,
		components:
			buttons.length > 0 ? [new ActionRowBuilder<ButtonBuilder>().addComponents(...buttons)] : [],
		allowedMentions: {},
	};
};

const reasonOf = (request: MeetRequest) =>
	request.declineReason ? `: ${request.declineReason}` : "";

export const meetRequestCard = (request: MeetRequest, outcome: MeetRequestOutcome) => {
	const head = [
		`${userMention(request.hostId)} wants to post this in ${channelMention(request.forumId)}`,
		request.hostEdited ? "-# Edited by the host" : "",
		outcome.kind === "open" && !request.dmChannelId
			? "-# Their DMs are closed, so I cannot tell them what happens."
			: "",
	];
	if (outcome.kind === "posted")
		return withPreview(request, [
			...head,
			`✅ Posted by ${userMention(outcome.by)}: ${channelMention(outcome.threadId)}`,
		]);
	if (outcome.kind === "withdrawn")
		return withPreview(request, [...head, "↩️ Withdrawn by the host"]);
	if (outcome.kind === "expired") return withPreview(request, [...head, EXPIRED]);
	if (request.state === "declined")
		return withPreview(request, [
			...head,
			`❌ Declined by ${userMention(request.handledBy ?? "")}${reasonOf(request)}`,
		]);
	return withPreview(request, head, [
		button(meetRequestCustomId("post", request.id), "Post", ButtonStyle.Primary),
		editButton(request, "Edit"),
		button(meetRequestCustomId("decline", request.id), "Decline", ButtonStyle.Danger),
	]);
};

export const meetRequestHostView = (
	request: MeetRequest,
	outcome: MeetRequestOutcome,
	guildName: string,
) => {
	const moderators = `the moderators of ${channelMention(request.forumId)} in **${guildName}**`;
	if (outcome.kind === "posted")
		return withPreview(request, [
			`✅ Posted: ${channelMention(outcome.threadId)}${request.moderatorsEdited ? ", with changes by the moderators" : ""}`,
		]);
	if (outcome.kind === "withdrawn") return withPreview(request, ["↩️ You withdrew it."]);
	if (outcome.kind === "expired") return withPreview(request, [EXPIRED]);
	const withdraw = button(meetRequestCustomId("withdraw", request.id), "Withdraw");
	if (request.state === "declined")
		return withPreview(
			request,
			[`❌ Declined by ${moderators}${reasonOf(request)}`],
			[editButton(request, "Edit and send again"), withdraw],
		);
	return withPreview(
		request,
		[
			`Waiting for ${moderators}.`,
			request.dmChannelId
				? ""
				: "-# Your DMs are closed, so this is the only place you can follow it.",
		],
		[editButton(request, "Edit"), withdraw],
	);
};
