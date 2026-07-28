import type {
	ChatInputCommandInteraction,
	ClientEvents,
	ContextMenuCommandBuilder,
	SlashCommandBuilder,
	UserContextMenuCommandInteraction,
} from "discord.js";

export interface CommandTypes {
	slash: {
		interaction: ChatInputCommandInteraction;
	};
	user: {
		interaction: UserContextMenuCommandInteraction;
	};
	menu: {
		interaction: UserContextMenuCommandInteraction;
	};
}

export interface CommandModuleDefault<Type extends keyof CommandTypes> {
	type: Type;
	name: string;
	data: Type extends "slash" ? SlashCommandBuilder : ContextMenuCommandBuilder;
	execute: (interaction: CommandTypes[Type]["interaction"]) => Promise<void>;
}

export interface EventModuleDefault<EventType extends keyof ClientEvents = keyof ClientEvents> {
	type: EventType;
	on: (...args: ClientEvents[EventType]) => Promise<void>;
}
