import {
	ActionRowBuilder,
	ButtonBuilder,
	ButtonStyle,
	channelMention,
	userMention,
} from "discord.js";
import type { MeetRequest } from "../db/schema.js";
import { MEET_FORM_BUTTON, MESSAGE_MAX, meetPostContent } from "./meet-post.js";

export const MEET_REQUEST_BUTTON = "meet-request";

export type MeetRequestAction = "post" | "decline" | "decline-reason" | "withdraw";

export const meetRequestCustomId = (action: MeetRequestAction, id: string) =>
	`${MEET_REQUEST_BUTTON}:${action}:${id}`;

export const meetRequestActionOf = (customId: string) => {
	const [prefix, action, id] = customId.split(":");
	if (prefix !== MEET_REQUEST_BUTTON || !action || !id) return null;
	return { action: action as MeetRequestAction, id };
};

export const meetFormTargetOfRequest = (id: string) => `request:${id}`;

export type MeetRequestOutcome =
	| { kind: "open" }
	| { kind: "posted"; by: string; threadId: string }
	| { kind: "withdrawn" }
	| { kind: "expired" };

const EXPIRED = "⌛ Nobody got to it before the meet would have been over.";

const button = (customId: string, label: string, style: ButtonStyle) =>
	new ButtonBuilder().setCustomId(customId).setLabel(label).setStyle(style);

const editButton = (request: MeetRequest, label: string) =>
	button(
		`${MEET_FORM_BUTTON}:${meetFormTargetOfRequest(request.id)}`,
		label,
		ButtonStyle.Secondary,
	);

const withPreview = (request: MeetRequest, head: string[], buttons: ButtonBuilder[]) => {
	const top = head.join("\n");
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

const editedNote = (request: MeetRequest) => {
	if (request.hostEdited && request.moderatorsEdited)
		return "-# Edited by the host and by the moderators";
	if (request.hostEdited) return "-# Edited by the host";
	if (request.moderatorsEdited) return "-# Edited by the moderators";
	return null;
};

const declinedLine = (request: MeetRequest) =>
	`❌ Declined by ${userMention(request.handledBy ?? "")}${request.declineReason ? `: ${request.declineReason}` : ""}`;

export const meetRequestCard = (request: MeetRequest, outcome: MeetRequestOutcome) => {
	const head = [
		`${userMention(request.hostId)} wants to post this in ${channelMention(request.forumId)}`,
	];
	const edited = editedNote(request);
	if (edited) head.push(edited);
	if (outcome.kind === "open" && request.dmChannelId === null)
		head.push("-# Their DMs are closed, so I cannot tell them what happens.");

	if (outcome.kind === "posted")
		return withPreview(
			request,
			[...head, `✅ Posted by ${userMention(outcome.by)}: ${channelMention(outcome.threadId)}`],
			[],
		);
	if (outcome.kind === "withdrawn")
		return withPreview(request, [...head, "↩️ Withdrawn by the host"], []);
	if (outcome.kind === "expired")
		return withPreview(
			request,
			[...head, request.state === "declined" ? declinedLine(request) : EXPIRED],
			[],
		);
	if (request.state === "posting")
		return withPreview(
			request,
			[...head, `-# ${userMention(request.handledBy ?? "")} is posting it`],
			[],
		);

	const post = button(
		meetRequestCustomId("post", request.id),
		request.state === "declined" ? "Post anyway" : "Post",
		ButtonStyle.Primary,
	);
	if (request.state === "declined")
		return withPreview(
			request,
			[...head, declinedLine(request)],
			[post, editButton(request, "Edit")],
		);
	return withPreview(request, head, [
		post,
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
		return withPreview(
			request,
			[
				`✅ Posted: ${channelMention(outcome.threadId)}${request.moderatorsEdited ? ", with changes by the moderators" : ""}`,
			],
			[],
		);
	if (outcome.kind === "withdrawn") return withPreview(request, ["↩️ You withdrew it."], []);
	const declined = `❌ Declined by ${moderators}${request.declineReason ? `: ${request.declineReason}` : ""}`;
	if (outcome.kind === "expired")
		return withPreview(request, [request.state === "declined" ? declined : EXPIRED], []);
	if (request.state === "posting")
		return withPreview(request, ["The moderators are posting it."], []);

	const withdraw = button(
		meetRequestCustomId("withdraw", request.id),
		"Withdraw",
		ButtonStyle.Secondary,
	);
	if (request.state === "declined")
		return withPreview(request, [declined], [editButton(request, "Edit and send again"), withdraw]);
	const head = [`Waiting for ${moderators}.`];
	if (request.moderatorsEdited) head.push("-# The moderators edited it.");
	if (request.dmChannelId === null)
		head.push("-# Your DMs are closed, so this is the only place you can follow it.");
	return withPreview(request, head, [editButton(request, "Edit"), withdraw]);
};
