import type { Role } from "discord.js";
import { html } from "hono/html";
import type { ForumConfig, GuildConfigChange } from "../core/guild-config.js";
import { forumIndexUrl } from "../lib/config.js";
import { page } from "./page.js";

export type ConfigPageOptions = {
	guildName: string;
	forums: ForumConfig[];
	roles: Role[];
	saved: boolean;
};

const MEET_FIELD = "meet";
const ANNOUNCE_FIELD_PREFIX = "announce:";

export const guildConfigChangeOfForm = (form: Record<string, unknown>): GuildConfigChange => ({
	meetForumIds: new Set(
		[form[MEET_FIELD] ?? []].flat().filter((id): id is string => typeof id === "string"),
	),
	announceRoleIdOf: new Map(
		Object.entries(form).flatMap(([field, value]) =>
			field.startsWith(ANNOUNCE_FIELD_PREFIX) && typeof value === "string"
				? [[field.slice(ANNOUNCE_FIELD_PREFIX.length), value] as const]
				: [],
		),
	),
});

const forumNotes = (forum: ForumConfig) =>
	[
		forum.name === null && "deleted",
		forum.name !== null && !forum.everyoneCanView && "hidden from @everyone",
		forum.isMeetForum && forum.name !== null && !forum.hasDefaultReaction && "no default reaction",
	].filter(Boolean);

const forumRow = (forum: ForumConfig, roles: Role[]) => html` <tr>
	<td>
		<label for="meet-${forum.forumId}"
			>${forum.name === null ? forum.forumId : `#${forum.name}`}</label
		>
		${forumNotes(forum).map((note) => html`<br /><span class="hint">${note}</span>`)}
		${forum.isMeetForum &&
		forum.everyoneCanView &&
		html`<br /><a class="hint" href="${forumIndexUrl(forum.forumId)}">Open Evnt index</a>`}
	</td>
	<td>
		<input
			type="checkbox"
			id="meet-${forum.forumId}"
			name="${MEET_FIELD}"
			value="${forum.forumId}"
			${forum.isMeetForum && html`checked`}
		/>
	</td>
	<td>
		<select
			name="${ANNOUNCE_FIELD_PREFIX}${forum.forumId}"
			aria-label="Announce role for ${forum.name ?? forum.forumId}"
		>
			<option value="">None</option>
			${roles.map(
				(role) =>
					html`<option value="${role.id}" ${role.id === forum.announceRoleId && html`selected`}>
						@${role.name}
					</option>`,
			)}
		</select>
	</td>
</tr>`;

export const configPage = ({ guildName, forums, roles, saved }: ConfigPageOptions) =>
	page(
		`ProtoCal · ${guildName}`,
		html`<h1>ProtoCal · ${guildName}</h1>
			${saved && html`<p>Saved.</p>`}
			<form method="post">
				<h2>Meet forums</h2>
				<p class="hint">
					Every new post in a meet forum becomes a meet. The announce role is pinged when one is
					posted. Members react with the forum's default reaction to attend.
				</p>
				${forums.length === 0
					? html`<p>This server has no forums.</p>`
					: html`<table>
							<thead>
								<tr>
									<th>Forum</th>
									<th>Meets</th>
									<th>Announce role</th>
								</tr>
							</thead>
							<tbody>
								${forums.map((forum) => forumRow(forum, roles))}
							</tbody>
						</table>`}
				<button>Save</button>
			</form>`,
	);
