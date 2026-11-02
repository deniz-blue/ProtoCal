import assert from "node:assert/strict";
import { test } from "node:test";
import type { OpenEvnt } from "@evnt/schema";
import {
	fieldsOfMeetDocument,
	meetChangeNotice,
	meetDatesAsDiscord,
	meetDocumentOfFields,
	meetIsOverAt,
	newMeetDocument,
	partialDateOfWritten,
	type MeetFields,
} from "./meet-document.ts";

const ID = "http://localhost:8660/events/1.evnt.json";
const unix = (iso: string) => Date.parse(iso) / 1000;
const fields = (changed: Partial<MeetFields>): MeetFields => ({
	name: "Bar meet",
	start: null,
	end: null,
	place: null,
	description: null,
	...changed,
});
const documentOf = (changed: Partial<MeetFields>, base = newMeetDocument(ID, "Bar meet")) => {
	const result = meetDocumentOfFields(fields(changed), base);
	assert.ok("document" in result, JSON.stringify(result));
	return result.document;
};
const problemOf = (changed: Partial<MeetFields>) => {
	const result = meetDocumentOfFields(fields(changed), newMeetDocument(ID, "Bar meet"));
	assert.ok("problem" in result);
	return result.problem;
};

test("written dates become Vilnius partial dates at the precision written", () => {
	assert.equal(partialDateOfWritten("2026-12-05 18:00"), "2026-12-05T18:00[Europe/Vilnius]");
	assert.equal(partialDateOfWritten(" 2026-12-05T18:00 "), "2026-12-05T18:00[Europe/Vilnius]");
	assert.equal(partialDateOfWritten("2026-12-05"), "2026-12-05[Europe/Vilnius]");
	assert.equal(partialDateOfWritten("2026-12"), "2026-12[Europe/Vilnius]");
	assert.equal(
		partialDateOfWritten("2026-12-05 18:00 [Europe/London]"),
		"2026-12-05T18:00[Europe/London]",
	);
	for (const written of ["05/12/2026", "2026-12-05 6pm", "saturday", "2026-12-05 18"])
		assert.equal(partialDateOfWritten(written), null, written);
});

test("the form makes a valid Open Evnt document", () => {
	assert.deepEqual(
		documentOf({
			start: "2026-12-05 18:00",
			end: "2026-12-06 02:00",
			place: "Snekutis, Vilnius",
			description: "Bring snacks",
		}),
		{
			v: "0.2",
			id: ID,
			name: { und: "Bar meet" },
			venue: { address: "Snekutis, Vilnius" },
			occurrences: [
				{ start: "2026-12-05T18:00[Europe/Vilnius]", end: "2026-12-06T02:00[Europe/Vilnius]" },
			],
			components: [{ $type: "directory.evnt.text.markdown#0", content: { und: "Bring snacks" } }],
		},
	);
});

test("dates that do not exist, or an end before the start, are refused with the field", () => {
	assert.equal(problemOf({ start: "2026-02-30" }).field, "start");
	assert.equal(problemOf({ start: "2026-12-05 18:00", end: "2026-12-05 17:00" }).field, "start");
	assert.equal(problemOf({ end: "2026-12-05" }).field, "start");
	assert.equal(problemOf({ start: "next friday" }).field, "start");
});

test("the form shows what the document holds and saving it unchanged keeps the document", () => {
	const fromAnApp: OpenEvnt = {
		v: "0.2",
		id: ID,
		name: { en: "Bar meet", lt: "Baro susitikimas" },
		venue: { name: { en: "Snekutis" }, address: "Stiklių g. 7, Vilnius", countryCode: "LT" },
		occurrences: [
			{ start: "2026-12-05T18:00[Europe/Vilnius]" },
			{ start: "2026-12-12T18:00[Europe/Vilnius]" },
		],
		components: [{ $type: "directory.evnt.link#0", url: "https://example.com" } as never],
	};
	const shown = fieldsOfMeetDocument(fromAnApp);
	assert.deepEqual(shown, {
		name: "Bar meet",
		start: "2026-12-05 18:00",
		end: null,
		place: "Stiklių g. 7, Vilnius",
		description: null,
	});
	const result = meetDocumentOfFields(shown, fromAnApp);
	assert.ok("document" in result);
	assert.deepEqual(result.document, fromAnApp);
});

test("a changed start keeps the other dates, and a changed place keeps the venue name", () => {
	const base = documentOf({ start: "2026-12-05 18:00" });
	const twoDates = {
		...base,
		occurrences: [...base.occurrences!, { start: "2026-12-12T18:00[Europe/Vilnius]" as const }],
	};
	const moved = documentOf({ start: "2026-12-05 19:00" }, twoDates);
	assert.deepEqual(moved.occurrences, [
		{ start: "2026-12-05T19:00[Europe/Vilnius]" },
		{ start: "2026-12-12T18:00[Europe/Vilnius]" },
	]);
	const named = { ...base, venue: { name: { en: "Snekutis" }, address: "Stiklių g. 7" } };
	assert.deepEqual(documentOf({ start: "2026-12-05 18:00", place: "Totorių g. 1" }, named).venue, {
		name: { en: "Snekutis" },
		address: "Totorių g. 1",
	});
});

test("dates read in Discord: a time, a whole day, several days, a month", () => {
	const at = unix("2026-12-05T16:00:00Z");
	assert.equal(
		meetDatesAsDiscord(documentOf({ start: "2026-12-05 18:00" })),
		`<t:${at}:f> <t:${at}:R>`,
	);
	assert.equal(
		meetDatesAsDiscord(documentOf({ start: "2026-12-05 18:00", end: "2026-12-05 23:00" })),
		`<t:${at}:f> – <t:${unix("2026-12-05T21:00:00Z")}:t> <t:${at}:R>`,
	);
	assert.equal(
		meetDatesAsDiscord(documentOf({ start: "2026-12-05", end: "2026-12-07" }), false),
		"Sat, 5 December 2026 – Mon, 7 December 2026",
	);
	assert.equal(meetDatesAsDiscord(documentOf({ start: "2026-12" })), "December 2026");
	assert.equal(meetDatesAsDiscord(documentOf({})), null);
});

test("a meet is over a day after it ends, or after its start when it has no end", () => {
	assert.equal(
		meetIsOverAt(documentOf({ start: "2026-12-05 18:00" })),
		unix("2026-12-06T16:00:00Z"),
	);
	assert.equal(meetIsOverAt(documentOf({ start: "2026-12-05" })), unix("2026-12-06T22:00:00Z"));
	assert.equal(
		meetIsOverAt(documentOf({ start: "2026-12-05", end: "2026-12-07" })),
		unix("2026-12-08T22:00:00Z"),
	);
	assert.equal(meetIsOverAt(documentOf({})), null);
});

test("a moved time or place pings, a description change does not, no change says nothing", () => {
	const before = documentOf({ start: "2026-12-05 18:00", place: "Bar", description: "Snacks" });
	const moved = meetChangeNotice(
		before,
		documentOf({ start: "2026-12-05 19:00", place: "Park", description: "Snacks" }, before),
	);
	assert.deepEqual(moved?.lines, [
		`📆 ~~<t:${unix("2026-12-05T16:00:00Z")}:f>~~ <t:${unix("2026-12-05T17:00:00Z")}:f>`,
		"📍 ~~Bar~~ Park",
	]);
	assert.equal(moved?.pingsRole, true);
	assert.deepEqual(
		meetChangeNotice(
			before,
			documentOf({ start: "2026-12-05 18:00", place: "Bar", description: "More" }, before),
		),
		{ lines: ["-# The details were updated."], pingsRole: false },
	);
	assert.equal(meetChangeNotice(before, before), null);
	assert.deepEqual(meetChangeNotice(before, { ...before, status: "cancelled" })?.lines, [
		"❌ **Cancelled**",
	]);
});
