import { ActionRowBuilder, ButtonBuilder, ButtonStyle, userMention } from "discord.js";
import type { OpenEvnt } from "@evnt/schema";
import { VIEWER_URL, meetDocumentUrl } from "../lib/config.js";
import {
	meetDatesAsDiscord,
	meetDescriptionOf,
	meetNameOf,
	meetPlaceOf,
	meetStatusAsDiscord,
} from "./meet-document.js";

// Discord limits: a message is 2000 characters, a thread name 100.
export const MESSAGE_MAX = 2000;
export const THREAD_NAME_MAX = 100;

export const MEET_FORM_BUTTON = "meet-form";
export const MEET_EDIT_BUTTON = "meet-edit";
export const MEET_FORUM_PICK = "meet-forum-pick";

export const meetFormTargetOfNew = (forumId: string) => `new:${forumId}`;
export const meetFormTargetOfEdit = (threadId: string) => `edit:${threadId}`;
export const meetFormTargetOfRequest = (requestId: string) => `request:${requestId}`;

export const threadNameOf = (document: OpenEvnt) => meetNameOf(document).slice(0, THREAD_NAME_MAX);

export const meetPostContent = (
	document: OpenEvnt,
	organizers: string[],
	updatedAtUnixSeconds: number,
	maxLength = MESSAGE_MAX,
) => {
	const dates = meetDatesAsDiscord(document);
	const place = meetPlaceOf(document);
	const facts = [
		meetStatusAsDiscord(document),
		dates && `📆 **Date & Time:** ${dates}`,
		place && `📍 **Location:** ${place}`,
		`🪧 **Organized by** ${organizers.map(userMention).join(" ")}`,
	].filter(Boolean);
	const head = [`## ${meetNameOf(document)}`, facts.join("\n")].join("\n\n");
	const foot = `-# Last updated at <t:${updatedAtUnixSeconds}:R>`;
	const description = meetDescriptionOf(document);
	if (!description) return [head, foot].join("\n\n");
	const room = maxLength - head.length - foot.length - 4;
	const shown = description.length > room ? `${description.slice(0, room - 1)}…` : description;
	return [head, shown, foot].join("\n\n");
};

export const meetViewUrl = (threadId: string) => {
	const viewer = new URL(VIEWER_URL);
	viewer.searchParams.set("url", meetDocumentUrl(threadId));
	return viewer.toString();
};

export const meetPostButtons = (threadId: string, viewable: boolean) => {
	const edit = new ButtonBuilder()
		.setCustomId(`${MEET_EDIT_BUTTON}:${threadId}`)
		.setLabel("Edit")
		.setStyle(ButtonStyle.Secondary);
	const view = new ButtonBuilder()
		.setLabel("View")
		.setStyle(ButtonStyle.Link)
		.setURL(meetViewUrl(threadId));
	return new ActionRowBuilder<ButtonBuilder>().addComponents(...(viewable ? [view, edit] : [edit]));
};

export const meetEditButtons = (threadId: string, editUrl: string) =>
	new ActionRowBuilder<ButtonBuilder>().addComponents(
		new ButtonBuilder()
			.setCustomId(`${MEET_FORM_BUTTON}:${meetFormTargetOfEdit(threadId)}`)
			.setLabel("Edit on Discord")
			.setStyle(ButtonStyle.Secondary),
		new ButtonBuilder().setLabel("Edit via App").setStyle(ButtonStyle.Link).setURL(editUrl),
	);
