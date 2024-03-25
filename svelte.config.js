import adapter from '@sveltejs/adapter-static' // This was changed from adapter-auto
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter(),
	},
}

export default config