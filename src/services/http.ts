import { serve } from "@hono/node-server";
import { HTTP_HOST, HTTP_PORT, PUBLIC_URL } from "../lib/config.js";
import { createChildLogger } from "../lib/logger.js";
import { app } from "../web/app.js";

const log = createChildLogger("http");

const server = serve({ fetch: app.fetch, hostname: HTTP_HOST, port: HTTP_PORT }, () =>
	log.info({ host: HTTP_HOST, port: HTTP_PORT, publicUrl: PUBLIC_URL }, "Listening"),
);

server.on("error", (error) => log.error({ error }, "HTTP server failed"));

if (import.meta.hot) {
	import.meta.hot.accept();
	import.meta.hot.dispose(() => new Promise((done) => server.close(() => done(undefined))));
}
