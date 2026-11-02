import { createHash } from "node:crypto";
import type { Meet } from "../db/schema.js";

export const etagOf = (body: string) =>
	`W/"${createHash("sha256").update(body).digest("base64url").slice(0, 22)}"`;

export const meetDocumentBody = (meet: Meet) => JSON.stringify(meet.document);

const isoOfUnixSeconds = (seconds: number) => new Date(seconds * 1000).toISOString();

export type ListedMeet = { meet: Meet; href: string; name: string | null };

const newestThreadFirst = ({ meet: a }: ListedMeet, { meet: b }: ListedMeet) =>
	b.threadId.length - a.threadId.length || b.threadId.localeCompare(a.threadId);

export const openEvntIndexOf = (listed: ListedMeet[]) => ({
	version: 1,
	items: listed.toSorted(newestThreadFirst).map(({ meet, href, name }) => ({
		href,
		name: name ?? undefined,
		updatedAt: isoOfUnixSeconds(meet.updatedAt),
		etag: etagOf(meetDocumentBody(meet)),
	})),
});
