export class AppError extends Error {
	constructor(
		message: string,
		public readonly code: string,
		public readonly cause?: Error,
	) {
		super(message);
		this.name = "AppError";
	}
}

export class ValidationError extends AppError {
	constructor(message: string, cause?: Error) {
		super(message, "VALIDATION_ERROR", cause);
		this.name = "ValidationError";
	}
}

export class SourceError extends AppError {
	constructor(message: string, cause?: Error) {
		super(message, "SOURCE_ERROR", cause);
		this.name = "SourceError";
	}
}

export class DiscordError extends AppError {
	constructor(message: string, cause?: Error) {
		super(message, "DISCORD_ERROR", cause);
		this.name = "DiscordError";
	}
}

export class DatabaseError extends AppError {
	constructor(message: string, cause?: Error) {
		super(message, "DATABASE_ERROR", cause);
		this.name = "DatabaseError";
	}
}
