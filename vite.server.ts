import { createServer, type RunnableDevEnvironment } from "vite";

// @ts-ignore
globalThis.window = globalThis;

const viteServer = await createServer({
	configFile: "./vite.config.ts",
});

viteServer.bindCLIShortcuts({
	print: true,
});

const serverEnvironment = viteServer.environments.node! as RunnableDevEnvironment;

const entry = process.argv.find((v, i, a) => a[i - 1] === "--entry") || "main";

await serverEnvironment.runner.import(`./src/init.ts`);
await serverEnvironment.runner.import(`./src/entry.${entry}.ts`);
