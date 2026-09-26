import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import starlightLlmsTxt from 'starlight-llms-txt';
import starlightConfig from './starlight.config.mjs';

export default defineConfig({
  site: 'https://nxlang.org',
  integrations: [
    starlight({
      ...starlightConfig,
      plugins: [
        // The playground is a separate site under /playground on the same domain.
        starlightLinksValidator({ exclude: ['/playground', '/playground/**'] }),
        starlightLlmsTxt({
          projectName: 'NX',
          description:
            'NX is a typed language for markup, data and the logic between them, in one syntax that runs on .NET, in JavaScript and in the browser.',
          // One set per sidebar section, so llms.txt links each of them.
          customSets: [
            { label: 'Overview', paths: ['overview/**'], description: 'what NX is and why' },
            { label: 'Language Tour', paths: ['language-tour/**'], description: 'a guided tour of the language' },
            { label: 'Tutorials', paths: ['tutorials/**'], description: 'getting started, and building with NX' },
            { label: 'Reference', paths: ['reference/**'], description: 'the exact rules for every construct' },
            { label: 'Contributing', paths: ['contributing/**'], description: 'building NX from source and working on it' }
          ]
        })
      ]
    })
  ]
});
