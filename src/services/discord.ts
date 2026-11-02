import {
	ForumChannel,
	MessageFlags,
	type ChatInputCommandInteraction,
	type Guild,
} from "discord.js";
import { DiscordError } from "../lib/errors.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("discord");

export async function createForumPost(
	channel: ForumChannel,
	name: string,
	content: string,
): Promise<{ postId: string; messageId: string }> {
	log.info({ channel: channel.id, name }, "Creating forum post");

	const thread = await channel.threads.create({
		name,
		message: { content },
	});

	log.info({ postId: thread.id }, "Created forum post");
	return { postId: thread.id, messageId: thread.id };
}

export async function sendEphemeral(
	interaction: ChatInputCommandInteraction,
	content: string,
): Promise<void> {
	await interaction.reply({
		content,
		flags: MessageFlags.Ephemeral,
	});
}

export function getGuildFromInteraction(interaction: ChatInputCommandInteraction): Guild {
	const guild = interaction.guild;
	if (!guild) {
		throw new DiscordError("This command can only be used in a server");
	}
	return guild;
}
