import type { OpenEvnt } from "@evnt/schema";
import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const meetForums = sqliteTable("meet_forums", {
	forumId: text("forum_id").primaryKey(),
	guildId: text("guild_id").notNull(),
	announceRoleId: text("announce_role_id"),
	reviewChannelId: text("review_channel_id"),
});

export const meets = sqliteTable("meets", {
	threadId: text("thread_id").primaryKey(),
	guildId: text("guild_id").notNull(),
	forumId: text("forum_id").notNull(),
	roleId: text("role_id").notNull(),
	attendReaction: text("attend_reaction").notNull(),
	organizers: text("organizers", { mode: "json" }).$type<string[]>().notNull(),
	document: text("document", { mode: "json" }).$type<OpenEvnt>().notNull(),
	updatedAt: integer("updated_at").notNull(),
});

export type MeetForum = typeof meetForums.$inferSelect;
export type Meet = typeof meets.$inferSelect;

export const calendarLinks = sqliteTable("calendar_links", {
	userId: text("user_id").primaryKey(),
	token: text("token").notNull().unique(),
	createdAt: integer("created_at").notNull(),
});

export type CalendarLink = typeof calendarLinks.$inferSelect;

export const meetRequests = sqliteTable("meet_requests", {
	id: text("id").primaryKey(),
	guildId: text("guild_id").notNull(),
	forumId: text("forum_id").notNull(),
	hostId: text("host_id").notNull(),
	document: text("document", { mode: "json" }).$type<OpenEvnt>().notNull(),
	state: text("state", { enum: ["waiting", "posting", "declined"] }).notNull(),
	handledBy: text("handled_by"),
	declineReason: text("decline_reason"),
	hostEdited: integer("host_edited", { mode: "boolean" }).notNull(),
	moderatorsEdited: integer("moderators_edited", { mode: "boolean" }).notNull(),
	cardChannelId: text("card_channel_id").notNull(),
	cardMessageId: text("card_message_id").notNull(),
	dmChannelId: text("dm_channel_id"),
	dmMessageId: text("dm_message_id"),
	updatedAt: integer("updated_at").notNull(),
});

export type MeetRequest = typeof meetRequests.$inferSelect;
