import type { ClientEvents } from "discord.js";
import type { CommandModuleDefault, CommandTypes, EventModuleDefault } from "./module-types.js";

export const defineCommand = <Type extends keyof CommandTypes>(
	command: CommandModuleDefault<Type>,
) => command;

export const defineEvent = <EventType extends keyof ClientEvents>(
	event: EventModuleDefault<EventType>,
) => event;
