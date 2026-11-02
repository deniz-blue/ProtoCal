import type { OpenEvnt } from "@evnt/schema";
import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const meetForums = sqliteTable("meet_forums", {
	forumId: text("forum_id").primaryKey(),
	guildId: text("guild_id").notNull(),
	announceRoleId: text("announce_role_id"),
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
