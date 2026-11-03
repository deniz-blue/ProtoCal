import assert from "node:assert/strict";
import { test } from "node:test";
import type { MeetRequest } from "../db/schema.ts";
import {
	meetRequestActionOf,
	meetRequestCard,
	meetRequestCustomId,
	meetRequestHostView,
} from "./meet-request-view.ts";

const request = (change: Partial<MeetRequest> = {}): MeetRequest => ({
	id: "5f0c6a52-6d0e-4a8e-9a3b-6f3e2c1d0b9a",
	guildId: "1",
	forumId: "2",
	hostId: "3",
	document: { v: "0.2", id: "urn:uuid:5f0c6a52", name: { und: "Bar meet" } },
	state: "waiting",
	handledBy: null,
	declineReason: null,
	hostEdited: false,
	moderatorsEdited: false,
	cardChannelId: "4",
	cardMessageId: "5",
	dmChannelId: "6",
	dmMessageId: "7",
	updatedAt: 1_800_000_000,
	...change,
});

const labelsOf = (view: ReturnType<typeof meetRequestCard>) =>
	view.components.flatMap((row) =>
		row.toJSON().components.map((button) => "label" in button && button.label),
	);

test("a waiting request's card asks the moderators, and the host can edit or withdraw", () => {
	const card = meetRequestCard(request(), { kind: "open" });
	assert.match(card.content, /^<@3> wants to post this in <#2>\n\n## Bar meet/);
	assert.deepEqual(labelsOf(card), ["Post", "Edit", "Decline"]);
	assert.deepEqual(labelsOf(meetRequestHostView(request(), { kind: "open" }, "LTFur")), [
		"Edit",
		"Withdraw",
	]);
});

test("a declined request shows who and why, and can still be posted or sent again", () => {
	const declined = request({ state: "declined", handledBy: "8", declineReason: "no place yet" });
	const card = meetRequestCard(declined, { kind: "open" });
	assert.match(card.content, /❌ Declined by <@8>: no place yet/);
	assert.deepEqual(labelsOf(card), ["Post anyway", "Edit"]);
	const hostView = meetRequestHostView(declined, { kind: "open" }, "LTFur");
	assert.match(
		hostView.content,
		/^❌ Declined by the moderators of <#2> in \*\*LTFur\*\*: no place yet/,
	);
	assert.deepEqual(labelsOf(hostView), ["Edit and send again", "Withdraw"]);
});

test("a posted request links the post on both sides and has no buttons", () => {
	const posted = { kind: "posted", by: "8", threadId: "9" } as const;
	const edited = request({ moderatorsEdited: true });
	const card = meetRequestCard(edited, posted);
	assert.match(card.content, /✅ Posted by <@8>: <#9>/);
	assert.deepEqual(card.components, []);
	assert.match(
		meetRequestHostView(edited, posted, "LTFur").content,
		/^✅ Posted: <#9>, with changes by the moderators/,
	);
});

test("closed DMs are noted on the card and in the host's reply", () => {
	const closed = request({ dmChannelId: null, dmMessageId: null });
	assert.match(meetRequestCard(closed, { kind: "open" }).content, /DMs are closed/);
	assert.match(meetRequestHostView(closed, { kind: "open" }, "LTFur").content, /DMs are closed/);
});

test("a long description still fits one message under the card's head", () => {
	const long = request({
		hostEdited: true,
		moderatorsEdited: true,
		document: {
			v: "0.2",
			id: "urn:uuid:5f0c6a52",
			name: { und: "Bar meet" },
			components: [
				{
					$type: "directory.evnt.text.markdown#0",
					content: { und: "x".repeat(3000) },
				},
			],
		} as MeetRequest["document"],
	});
	assert.ok(meetRequestCard(long, { kind: "open" }).content.length <= 2000);
	assert.ok(meetRequestHostView(long, { kind: "open" }, "LTFur").content.length <= 2000);
});

test("button ids carry the action and the request", () => {
	const id = request().id;
	assert.deepEqual(meetRequestActionOf(meetRequestCustomId("withdraw", id)), {
		action: "withdraw",
		id,
	});
	assert.equal(meetRequestActionOf("meet-form:request:x"), null);
	assert.ok(meetRequestCustomId("decline-reason", id).length <= 100);
});
