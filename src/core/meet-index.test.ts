import assert from "node:assert/strict";
import { test } from "node:test";
import type { Meet } from "../db/schema.ts";
import { etagOf, meetDocumentBody, openEvntIndexOf } from "./meet-index.ts";

const meet = (threadId: string, name: string, updatedAt: number): Meet => ({
	threadId,
	guildId: "1",
	forumId: "2",
	roleId: "3",
	attendReaction: "👍",
	organizers: [],
	document: { v: "0.2", id: `https://p.test/events/${threadId}.evnt.json`, name: { und: name } },
	updatedAt,
});

test("an index lists the newest thread first, with each document's etag", () => {
	const older = meet("998", "Bar meet", 1_800_000_000);
	const newer = meet("1001", "Picnic", 1_800_000_060);
	const index = openEvntIndexOf([
		{ meet: older, href: "/e/998", name: "Bar meet" },
		{ meet: newer, href: "/e/1001", name: "Picnic" },
	]);
	assert.deepEqual(index, {
		version: 1,
		items: [
			{
				href: "/e/1001",
				name: "Picnic",
				updatedAt: "2027-01-15T08:01:00.000Z",
				etag: etagOf(meetDocumentBody(newer)),
			},
			{
				href: "/e/998",
				name: "Bar meet",
				updatedAt: "2027-01-15T08:00:00.000Z",
				etag: etagOf(meetDocumentBody(older)),
			},
		],
	});
});

test("etags change with the body and are weak", () => {
	assert.match(etagOf("a"), /^W\/"[A-Za-z0-9_-]{22}"$/);
	assert.notEqual(etagOf("a"), etagOf("b"));
});
