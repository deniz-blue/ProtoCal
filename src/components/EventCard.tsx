import type { OpenEvnt } from "@evnt/schema";
import { DiscordFormatter } from "@evnt/pretty";

export const EventCard = ({ event }: { event: OpenEvnt }) => {
	const summary = new DiscordFormatter({
		...DiscordFormatter.discordDefaults,
	}).formatEvent(event);

	return (
		<container>
			<text>{summary}</text>
		</container>
	);
};
