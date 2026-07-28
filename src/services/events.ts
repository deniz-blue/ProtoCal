import type { EventModuleDefault } from "../core/module-types.js";
import { client } from "./client.js";

export let events: EventModuleDefault[] = Object.values(
	import.meta.glob("../events/**/*.*", { eager: true }),
).map((loader) => (loader as any).default as EventModuleDefault);

for (const { type, on } of events) client.on(type, on);

console.log(
	"Loaded events: ",
	events.map((c) => c.type),
);

if (import.meta.hot) {
	import.meta.hot.accept((newModule) => {
		if (!newModule) return console.error("Failed to load new module for events");

		let newEvents = (newModule as any).events as EventModuleDefault[];

		for (const { type, on } of events) client.off(type, on);

		for (const { type, on } of newEvents) client.on(type, on);

		events = newEvents;
		console.log("Events reloaded");
	});
}
