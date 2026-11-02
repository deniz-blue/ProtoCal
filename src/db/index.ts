import { createClient } from "@libsql/client";
import { drizzle, type LibSQLDatabase } from "drizzle-orm/libsql";
import * as schema from "./schema.js";

type Schema = typeof schema;

let db: LibSQLDatabase<Schema> | null = null;

export function getDb(): LibSQLDatabase<Schema> {
	if (db) return db;

	const client = createClient({
		url: process.env.DATABASE_URL ?? "file:./data/protocal.db",
	});

	db = drizzle(client, { schema });
	return db;
}
