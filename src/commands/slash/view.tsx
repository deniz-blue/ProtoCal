import { SlashCommandBuilder } from "discord.js";
import { defineCommand } from "../../core/define.js";
import { djsx } from "discord-jsx-renderer";
import { EventCard } from "../../components/EventCard.js";

export default defineCommand({
	type: "slash",
	name: "view",
	data: new SlashCommandBuilder().setName("view").setDescription("View an event"),
	execute: async (interaction) => {
		djsx.createMessage(
			interaction,
			<message>
				<EventCard
					event={{
						v: "0.1",
						name: {
							en: "Keynote: The Future of Events",
						},
						instances: [
							{
								venueIds: [],
								start: "2027-03-08T09:00[UTC]",
								end: "2027-03-08T10:00[UTC]",
							},
						],
						components: [{ $type: "directory.evnt.component.link", url: "https://example.com" }],
					}}
				/>
			</message>,
		);
	},
});
