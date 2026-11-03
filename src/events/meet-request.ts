import { Events } from "discord.js";
import { defineEvent } from "../core/define.js";
import {
	askDeclineReason,
	declineMeetRequest,
	postMeetRequest,
	withdrawMeetRequest,
} from "../core/meet-request.js";
import { meetRequestActionOf } from "../core/meet-request-view.js";

export default defineEvent({
	type: Events.InteractionCreate,
	on: async (interaction) => {
		if (!interaction.isButton() && !interaction.isModalSubmit()) return;
		const pressed = meetRequestActionOf(interaction.customId);
		if (!pressed) return;
		if (interaction.isModalSubmit()) {
			if (pressed.action === "decline-reason") await declineMeetRequest(interaction, pressed.id);
			return;
		}
		if (pressed.action === "post") await postMeetRequest(interaction, pressed.id);
		else if (pressed.action === "decline") await askDeclineReason(interaction, pressed.id);
		else if (pressed.action === "withdraw") await withdrawMeetRequest(interaction, pressed.id);
	},
});
