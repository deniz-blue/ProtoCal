import {
	ChannelType,
	InteractionContextType,
	MessageFlags,
	PermissionFlagsBits,
	SlashCommandBuilder,
	type ChatInputCommandInteraction,
} from "discord.js";
import { defineCommand } from "../../core/define.js";
import { offerNewMeetForm } from "../../core/meet-form.js";
import { startMeet } from "../../core/meet.js";
import { meetForumOf, meetOfThread } from "../../db/meets.js";
import { isScreened } from "../../core/meet-access.js";

const trackPost = async (interaction: ChatInputCommandInteraction) => {
	const reply = (content: string) => interaction.editReply({ content, allowedMentions: {} });
	await interaction.deferReply({ flags: MessageFlags.Ephemeral });

	const thread = interaction.channel;
	if (!thread?.isThread()) {
		await reply("Run this inside the post.");
		return;
	}
	const isStaff = interaction.memberPermissions?.has(PermissionFlagsBits.ManageThreads) ?? false;
	const forum = thread.parentId ? await meetForumOf(thread.parentId) : null;
	if (forum && isScreened(forum) && !isStaff) {
		await reply("Moderators check every meet in this forum, so only staff can do this.");
		return;
	}
	if (thread.ownerId !== interaction.user.id && !isStaff) {
		await reply("Only the person who posted it or staff can do this.");
		return;
	}
	if (await meetOfThread(thread.id)) {
		await reply("This post is a meet already.");
		return;
	}

	const started = await startMeet(thread);
	await reply(
		started === "started"
			? "This post is a meet now. Everyone who already reacted got the role."
			: started === "no-attend-reaction"
				? "This forum has no default reaction to attend with. Staff can set one in the forum's settings."
				: "This post is not in a meet forum.",
	);
};

export default defineCommand({
	type: "slash",
	name: "event",
	data: new SlashCommandBuilder()
		.setName("event")
		.setDescription("Meets")
		.setContexts(InteractionContextType.Guild)
		.addSubcommand((sub) =>
			sub
				.setName("new")
				.setDescription("Post a meet")
				.addChannelOption((option) =>
					option
						.setName("forum")
						.setDescription("Where to post it; without it you pick from the forums you can post in")
						.addChannelTypes(ChannelType.GuildForum),
				),
		)
		.addSubcommand((sub) =>
			sub.setName("track").setDescription("Make this post, made by hand, a meet"),
		) as SlashCommandBuilder,
	execute: async (interaction) => {
		if (interaction.options.getSubcommand() === "track") return trackPost(interaction);

		await offerNewMeetForm(interaction);
	},
});
