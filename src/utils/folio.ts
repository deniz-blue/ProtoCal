import type { OpenEvnt } from "@evnt/schema";

const { FOLIO_API_ENDPOINT, FOLIO_API_SECRET } = process.env;

const headers = {
	"Content-Type": "application/json",
	Authorization: `Bearer ${FOLIO_API_SECRET}`,
};

export const Folio = {
	enabled: FOLIO_API_ENDPOINT !== undefined,

	get: async (id: string): Promise<OpenEvnt> => {
		const res = await fetch(new URL(`/events/${id}`, FOLIO_API_ENDPOINT), {
			method: "GET",
			headers,
		});

		if (!res.ok)
			throw new Error(`Failed to fetch event from Folio: ${res.status} ${res.statusText}`);

		return (await res.json()) as OpenEvnt;
	},

	create: async (data: OpenEvnt): Promise<string> => {
		const res = await fetch(new URL("/events", FOLIO_API_ENDPOINT), {
			method: "POST",
			headers,
			body: JSON.stringify(data),
		});

		if (!res.ok)
			throw new Error(`Failed to create event in Folio: ${res.status} ${res.statusText}`);

		const { id } = (await res.json()) as { id: string };
		return id;
	},

	put: async (id: string, data: OpenEvnt): Promise<void> => {
		const res = await fetch(new URL(`/events/${id}`, FOLIO_API_ENDPOINT), {
			method: "PUT",
			headers,
			body: JSON.stringify(data),
		});

		if (!res.ok)
			throw new Error(`Failed to update event in Folio: ${res.status} ${res.statusText}`);
	},
};
