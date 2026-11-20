import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

export default defineConfig({
	plugins: [solidPlugin()],
	resolve: {
		alias: {
			"@components": "/app/components",
			"@hooks": "/app/hooks",
			"@utils": "/app/utils",
			"@assets": "/app/assets",
			"@tutorials": "/app/tutorials",
			"@examples": "/app/examples",
		},
	},
	worker: {
		format: "es",
	},
	build: {
		target: "esnext",
		rollupOptions: {
			output: {
				// Keep the editor libraries in their own long-lived cacheable chunk.
				manualChunks(id) {
					const core =
						/node_modules\/.*\/(@codemirror\/(state|view|language|commands|autocomplete|lint|search)|@lezer\/(common|lr|highlight)|style-mod|w3c-keyname|crelt)\//;
					if (core.test(id)) return "codemirror";
				},
			},
		},
	},
	server: {
		port: 3000,
	},
});
