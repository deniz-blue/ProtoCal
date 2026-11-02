import assert from "node:assert/strict";
import { test } from "node:test";
import { reactionAsText, reactionKey, roleNameOfMeet } from "./attend-reaction.ts";

test("a custom emoji is known by its id, a unicode one by itself", () => {
	assert.equal(reactionKey({ id: "1550491800844636291", name: "hello" }), "1550491800844636291");
	assert.equal(reactionKey({ id: null, name: "👋" }), "👋");
	assert.equal(reactionKey({ id: null, name: null }), null);
});

test("an emoji is written the way Discord shows it in a message", () => {
	assert.equal(
		reactionAsText({ id: "1550491800844636291", name: "hello" }),
		"<:hello:1550491800844636291>",
	);
	assert.equal(reactionAsText({ id: null, name: "👋" }), "👋");
});

test("a role name is cut to what Discord accepts", () => {
	assert.equal(roleNameOfMeet("Bar meet"), "Meet: Bar meet");
	assert.equal(roleNameOfMeet("x".repeat(200)).length, 100);
});
