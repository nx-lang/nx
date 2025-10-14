import { defineConfig } from 'astro/config';
import { fileURLToPath } from 'node:url';
import starlight from '@astrojs/starlight';
import darkPlus from 'shiki/themes/dark-plus.mjs';
import lightPlus from 'shiki/themes/light-plus.mjs';
import starlightConfig from './starlight.config.mjs';

const nxGrammarPath = fileURLToPath(
  new URL('../src/vscode/syntaxes/nx.tmLanguage.json', import.meta.url)
);

export default defineConfig({
  integrations: [
    starlight(starlightConfig)
  ],
  site: 'https://nx-lang.github.io',
  base: '/nx',
  markdown: {
    shikiConfig: {
      themes: {
        dark: darkPlus,
        light: lightPlus
      },
      langs: [
        {
          name: 'nx',
          scopeName: 'source.nx',
          displayName: 'NX',
          path: nxGrammarPath
        }
      ]
    }
  }
});
