import type { Role, TextChannel } from "discord.js";
import { html } from "hono/html";
import type { ForumConfig, GuildConfigChange } from "../core/guild-config.js";
import { forumIndexUrl } from "../lib/config.js";
import { page } from "./page.js";

export type ConfigPageOptions = {
	guildName: string;
	forums: ForumConfig[];
	roles: Role[];
	reviewChannels: Pick<TextChannel, "id" | "name">[];
	saved: boolean;
};

const MEET_FIELD = "meet";
const ANNOUNCE_FIELD_PREFIX = "announce:";
const REVIEW_FIELD_PREFIX = "review:";

const valuesByForum = (form: Record<string, unknown>, prefix: string) =>
	new Map(
		Object.entries(form).flatMap(([field, value]) =>
			field.startsWith(prefix) && typeof value === "string"
				? [[field.slice(prefix.length), value] as const]
				: [],
		),
	);

export const guildConfigChangeOfForm = (form: Record<string, unknown>): GuildConfigChange => ({
	meetForumIds: new Set(
		[form[MEET_FIELD] ?? []].flat().filter((id): id is string => typeof id === "string"),
	),
	announceRoleIdOf: valuesByForum(form, ANNOUNCE_FIELD_PREFIX),
	reviewChannelIdOf: valuesByForum(form, REVIEW_FIELD_PREFIX),
});

const forumNotes = (forum: ForumConfig) =>
	[
		forum.name === null && "deleted",
		forum.name !== null && !forum.everyoneCanView && "hidden from @everyone",
		forum.isMeetForum && forum.name !== null && !forum.hasDefaultReaction && "no default reaction",
		forum.reviewChannelId !== null &&
			forum.everyoneCanPost &&
			"screened, but @everyone can still make posts by hand",
	].filter(Boolean);

const forumRow = (
	forum: ForumConfig,
	roles: Role[],
	reviewChannels: ConfigPageOptions["reviewChannels"],
) => html` <tr>
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
	<td>
		<select
			name="${REVIEW_FIELD_PREFIX}${forum.forumId}"
			aria-label="Review channel for ${forum.name ?? forum.forumId}"
		>
			<option value="">None</option>
			${reviewChannels.map(
				(channel) =>
					html`<option
						value="${channel.id}"
						${channel.id === forum.reviewChannelId && html`selected`}
					>
						#${channel.name}
					</option>`,
			)}
		</select>
	</td>
</tr>`;

export const configPage = ({
	guildName,
	forums,
	roles,
	reviewChannels,
	saved,
}: ConfigPageOptions) =>
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
				<p class="hint">
					With a review channel, a new meet goes there first, and anyone with Manage Threads in the
					forum posts, edits or declines it. Turn Create Posts off for members in a screened forum,
					so the bot's form is the only way in.
				</p>
				${forums.length === 0
					? html`<p>This server has no forums.</p>`
					: html`<table>
							<thead>
								<tr>
									<th>Forum</th>
									<th>Meets</th>
									<th>Announce role</th>
									<th>Review channel</th>
								</tr>
							</thead>
							<tbody>
								${forums.map((forum) => forumRow(forum, roles, reviewChannels))}
							</tbody>
						</table>`}
				<button>Save</button>
			</form>`,
	);
