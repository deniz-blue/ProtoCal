import pino from "pino";

const level = process.env.LOG_LEVEL ?? "info";

export const logger =
	process.env.NODE_ENV !== "production"
		? pino({
				level,
				transport: { target: "pino-pretty", options: { colorize: true } },
			})
		: pino({ level });

export function createChildLogger(name: string) {
	return logger.child({ module: name });
}
