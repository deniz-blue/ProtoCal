import type { CommandModuleDefault } from "../core/module-types.js";
import { createChildLogger } from "../lib/logger.js";

const log = createChildLogger("commands");

export let commands: CommandModuleDefault<any>[] = Object.values(
	import.meta.glob("../commands/**/*.*", { eager: true }),
).map((loader) => (loader as any).default as CommandModuleDefault<any>);

log.info({ commands: commands.map((c) => c.name) }, "Loaded commands");

if (import.meta.hot) {
	import.meta.hot.accept((newModule) => {
		if (!newModule) return log.error("Failed to load new module for commands");
		commands = (newModule as any).commands;
		log.info("Commands reloaded");
	});
}
