import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	appType: "custom",
	plugins: [react()],
	server: {
		middlewareMode: true,
	},
	clearScreen: false,
	envPrefix: ["VITE_", "NODE_", "DISCORD_"],
	environments: {
		node: {
			resolve: {
				conditions: ["node"],
			},
			optimizeDeps: {
				disabled: true,
			},
			keepProcessEnv: true,
			consumer: "server",
			build: {
				target: "NodeNext",
			},
		},
	},
});
