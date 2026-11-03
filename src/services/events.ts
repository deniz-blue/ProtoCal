import { BaseInteraction, MessageFlags } from "discord.js";
import type { EventModuleDefault } from "../core/module-types.js";
import { client } from "./client.js";
import { AppError } from "../lib/errors.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("events");

export const events: EventModuleDefault[] = Object.values(
	import.meta.glob("../events/**/*.*", { eager: true }),
).map((loader) => (loader as any).default as EventModuleDefault);

const answerFailedInteraction = async (interaction: BaseInteraction, error: unknown) => {
	if (!interaction.isRepliable()) return;
	const content = `❌ ${error instanceof AppError ? error.message : "An unexpected error occurred."}`;
	// discord.js leaves `ephemeral` null after deferUpdate, and editReply would then overwrite the clicked message.
	const deferredAsUpdate = interaction.deferred && interaction.ephemeral === null;
	if (interaction.deferred && !interaction.replied && !deferredAsUpdate)
		await interaction.editReply(content);
	else if (interaction.replied || deferredAsUpdate)
		await interaction.followUp({ content, flags: MessageFlags.Ephemeral });
	else await interaction.reply({ content, flags: MessageFlags.Ephemeral });
};

const listeners = events.map(({ type, on }) => {
	const listener: EventModuleDefault["on"] = (...args) =>
		on(...args).catch(async (error) => {
			log.error({ event: type, error }, "Event handler failed");
			const [subject] = args;
			if (subject instanceof BaseInteraction)
				await answerFailedInteraction(subject, error).catch((answerError) =>
					log.error({ error: answerError }, "Failed interaction was not answered"),
				);
		});
	client.on(type, listener);
	return { type, listener };
});

log.info({ events: events.map((c) => c.type) }, "Loaded events");

if (import.meta.hot) {
	import.meta.hot.accept();
	import.meta.hot.dispose(() => {
		for (const { type, listener } of listeners) client.off(type, listener);
		log.info("Events reloaded");
	});
}
