import { randomBytes } from "node:crypto";

const GRANT_LIFETIME_MS = 60 * 60 * 1000;

export const createGrantStore = <T extends object>() => {
	const grants = new Map<string, { payload: T; expiresAt: number }>();
	return {
		grant: (payload: T) => {
			const token = randomBytes(24).toString("base64url");
			grants.set(token, { payload, expiresAt: Date.now() + GRANT_LIFETIME_MS });
			return token;
		},
		find: (token: string): T | null => {
			const grant = grants.get(token);
			if (!grant) return null;
			if (grant.expiresAt > Date.now()) return grant.payload;
			grants.delete(token);
			return null;
		},
		spend: (token: string) => grants.delete(token),
	};
};
