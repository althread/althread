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
	build: {
		target: "esnext",
	},
	server: {
		port: 3000,
	},
});
