import { PartialDate } from "@evnt/partial-date";
import {
	OpenEvntValidator,
	type AnyComponent,
	type MarkdownTextComponent,
	type OpenEvnt,
	type Occurrence,
	type Translations,
	type Venue,
} from "@evnt/schema";

export const MEET_TIME_ZONE = "Europe/Vilnius";

const SECONDS_PER_DAY = 86400;
const DESCRIPTION_TYPE = "directory.evnt.text.markdown#0";
const SHOWN_LANGUAGES = ["mul", "und", "en", "lt"];

export type MeetFields = {
	name: string;
	start: string | null;
	end: string | null;
	place: string | null;
	description: string | null;
};

export type MeetFieldProblem = { field: keyof MeetFields | null; message: string };

export const newMeetDocument = (id: string, name: string): OpenEvnt => ({
	v: "0.2",
	id,
	name: { und: name },
});

export const textOf = (translations: Translations | undefined) => {
	if (!translations) return null;
	const language =
		SHOWN_LANGUAGES.find((candidate) => translations[candidate] !== undefined) ??
		Object.keys(translations)[0];
	return language === undefined ? null : translations[language]!;
};

const isDescription = (component: AnyComponent): component is MarkdownTextComponent =>
	component.$type === DESCRIPTION_TYPE;

export const meetDescriptionOf = (document: OpenEvnt) =>
	textOf(document.components?.find(isDescription)?.content);

export const meetPlaceOf = (document: OpenEvnt) =>
	document.venue ? (document.venue.address ?? textOf(document.venue.name)) : null;

export const meetNameOf = (document: OpenEvnt) => textOf(document.name) ?? "";

const WRITTEN_DATE =
	/^(\d{4})(?:-(\d{2})(?:-(\d{2})(?:[ T](\d{2}):(\d{2}))?)?)?(?:\s*\[([^\]\s]+)\])?$/;

export const partialDateOfWritten = (written: string) => {
	const parts = WRITTEN_DATE.exec(written.trim());
	if (!parts) return null;
	const [, year, month, day, hour, minute, timeZone] = parts;
	const date = [year, month, day].filter(Boolean).join("-");
	return `${date}${hour ? `T${hour}:${minute}` : ""}[${timeZone ?? MEET_TIME_ZONE}]`;
};

export const writtenOfPartialDate = (partialDate: string) => {
	const parts = /^([^[]+)\[(.+)\]$/.exec(partialDate);
	if (!parts) return partialDate;
	const written = parts[1]!.replace("T", " ");
	return parts[2] === MEET_TIME_ZONE ? written : `${written} [${parts[2]}]`;
};

export const fieldsOfMeetDocument = (document: OpenEvnt): MeetFields => {
	const first = document.occurrences?.[0];
	return {
		name: meetNameOf(document),
		start: first?.start ? writtenOfPartialDate(first.start) : null,
		end: first?.end ? writtenOfPartialDate(first.end) : null,
		place: meetPlaceOf(document),
		description: meetDescriptionOf(document),
	};
};

const withoutEmpty = <T extends object>(value: T) => {
	const kept = Object.fromEntries(
		Object.entries(value).filter(([, field]) => field !== undefined && field !== null),
	) as T;
	return Object.keys(kept).length > 0 ? kept : undefined;
};

const occurrencesWith = (
	occurrences: Occurrence[] | undefined,
	start: string | null,
	end: string | null,
) => {
	const [first, ...rest] = occurrences ?? [];
	if (start === (first?.start ?? null) && end === (first?.end ?? null)) return occurrences;
	const changed = withoutEmpty({ ...first, start, end } as Occurrence);
	const changedOccurrences = changed ? [changed, ...rest] : rest;
	return changedOccurrences.length > 0 ? changedOccurrences : undefined;
};

const venueWith = (venue: Venue | undefined, shownPlace: string | null, place: string | null) => {
	if (place === shownPlace) return venue;
	const { address, name, ...rest } = venue ?? {};
	return withoutEmpty({
		...rest,
		name: address !== undefined ? name : undefined,
		address: place ?? undefined,
	});
};

const componentsWith = (
	components: AnyComponent[] | undefined,
	shownDescription: string | null,
	description: string | null,
) => {
	if (description === shownDescription) return components;
	const others = (components ?? []).filter((component) => !isDescription(component));
	const changed = description
		? [{ $type: DESCRIPTION_TYPE, content: { und: description } } as AnyComponent, ...others]
		: others;
	return changed.length > 0 ? changed : undefined;
};

const FIELD_LABELS: Record<keyof MeetFields, string> = {
	name: "Name",
	start: "Start",
	end: "End",
	place: "Place",
	description: "Description",
};

const fieldOfIssuePath = (path: readonly unknown[]): keyof MeetFields | null => {
	const keys = path.map((segment) =>
		typeof segment === "object" && segment !== null && "key" in segment ? segment.key : segment,
	);
	if (keys[0] === "name") return "name";
	if (keys[0] === "venue") return "place";
	if (keys[0] === "components") return "description";
	if (keys[0] === "occurrences" && keys[1] === 0 && (keys[2] === "start" || keys[2] === "end"))
		return keys[2];
	return null;
};

export const validateMeetDocument = (
	document: unknown,
): { document: OpenEvnt } | { problem: MeetFieldProblem } => {
	const result = OpenEvntValidator.validate(document);
	if (!result.issues) return { document: result.value };
	const [issue] = result.issues;
	const field = fieldOfIssuePath(issue?.path ?? []);
	return {
		problem: {
			field,
			message: field ? `${FIELD_LABELS[field]} ${issue!.message}.` : `${issue!.message}.`,
		},
	};
};

const unreadableDate = (field: "start" | "end", written: string): MeetFieldProblem => ({
	field,
	message: `${FIELD_LABELS[field]}: \`${written}\` is not a date I can read. Write it as \`2026-12-05 18:00\`, \`2026-12-05\` for a whole day, or \`2026-12\` for a month.`,
});

export const meetDocumentOfFields = (
	fields: MeetFields,
	base: OpenEvnt,
): { document: OpenEvnt } | { problem: MeetFieldProblem } => {
	const start = fields.start ? partialDateOfWritten(fields.start) : null;
	if (fields.start && !start) return { problem: unreadableDate("start", fields.start) };
	const end = fields.end ? partialDateOfWritten(fields.end) : null;
	if (fields.end && !end) return { problem: unreadableDate("end", fields.end) };
	if (end && !start)
		return { problem: { field: "start", message: "An end needs a start as well." } };

	const shown = fieldsOfMeetDocument(base);
	const document = withoutEmpty({
		...base,
		name: fields.name === shown.name ? base.name : { und: fields.name },
		venue: venueWith(base.venue, shown.place, fields.place),
		occurrences: occurrencesWith(base.occurrences, start, end),
		components: componentsWith(base.components, shown.description, fields.description),
	} as OpenEvnt);
	return validateMeetDocument(document);
};

const unixSecondsOf = (date: PartialDate) => Math.floor(date.startEpochMilliseconds / 1000);

const dateAsDiscord = (date: PartialDate) => {
	if (date.precision === "time") return `<t:${unixSecondsOf(date)}:f>`;
	return new Intl.DateTimeFormat("en-GB", {
		timeZone: date.timeZoneId,
		year: "numeric",
		month: date.precision === "year" ? undefined : "long",
		day: date.precision === "day" ? "numeric" : undefined,
		weekday: date.precision === "day" ? "short" : undefined,
	}).format(date.startEpochMilliseconds);
};

const occurrenceAsDiscord = (occurrence: Occurrence, relative: boolean) => {
	if (!occurrence.start) return null;
	const start = PartialDate.from(occurrence.start);
	const until = occurrence.end && PartialDate.from(occurrence.end);
	const sameDay =
		until &&
		start.precision === "time" &&
		until.precision === "time" &&
		occurrence.start.slice(0, 10) === occurrence.end!.slice(0, 10) &&
		start.timeZoneId === until.timeZoneId;
	const range = !until
		? dateAsDiscord(start)
		: `${dateAsDiscord(start)} – ${sameDay ? `<t:${unixSecondsOf(until)}:t>` : dateAsDiscord(until)}`;
	const shown = relative && start.has("day") ? `${range} <t:${unixSecondsOf(start)}:R>` : range;
	return occurrence.status === "cancelled" ? `~~${shown}~~` : shown;
};

export const meetDatesAsDiscord = (document: OpenEvnt, relative = true) => {
	const dates = (document.occurrences ?? [])
		.map((occurrence) => occurrenceAsDiscord(occurrence, relative))
		.filter(Boolean);
	return dates.length > 0 ? dates.join(", ") : null;
};

export const meetIsOverAt = (document: OpenEvnt) => {
	const lastMoments = (document.occurrences ?? [])
		.map((occurrence) => occurrence.end ?? occurrence.start)
		.filter((date) => date !== undefined)
		.map((date) => PartialDate.from(date).endEpochMilliseconds / 1000);
	return lastMoments.length > 0 ? Math.max(...lastMoments) + SECONDS_PER_DAY : null;
};

const STATUS_TEXT: Record<NonNullable<OpenEvnt["status"]>, string> = {
	uncertain: "⚠️ **May not happen as listed**",
	cancelled: "❌ **Cancelled**",
	postponed: "⏸️ **Postponed**, new dates not known yet",
};

export const meetStatusAsDiscord = (document: OpenEvnt) =>
	document.status ? STATUS_TEXT[document.status] : null;

export type MeetChangeNotice = { lines: string[]; pingsRole: boolean };

const changeLine = (icon: string, before: string | null, after: string | null, unset: string) =>
	before ? `${icon} ~~${before}~~ ${after ?? unset}` : `${icon} ${after}`;

export const meetChangeNotice = (before: OpenEvnt, after: OpenEvnt): MeetChangeNotice | null => {
	const lines: string[] = [];
	if (before.status !== after.status)
		lines.push(meetStatusAsDiscord(after) ?? "✅ **On as listed again**");
	const datesBefore = meetDatesAsDiscord(before, false);
	const datesAfter = meetDatesAsDiscord(after, false);
	if (datesBefore !== datesAfter)
		lines.push(changeLine("📆", datesBefore, datesAfter, "no time yet"));
	const placeBefore = meetPlaceOf(before);
	const placeAfter = meetPlaceOf(after);
	if (placeBefore !== placeAfter)
		lines.push(changeLine("📍", placeBefore, placeAfter, "no place yet"));
	if (lines.length > 0) return { lines, pingsRole: true };
	if (JSON.stringify(before) !== JSON.stringify(after))
		return { lines: ["-# The details were updated."], pingsRole: false };
	return null;
};
