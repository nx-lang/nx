import nxGrammar from '../src/vscode/syntaxes/nx.tmLanguage.json' assert { type: 'json' };
import darkPlus from 'shiki/themes/dark-plus.mjs';
import lightPlus from 'shiki/themes/light-plus.mjs';

const nxLanguage = {
  ...nxGrammar,
  name: 'nx'
};

/** @type {import('@astrojs/starlight/types').StarlightUserConfig} */
const config = {
  title: 'NX Language',
  description: 'Official documentation for the NX language.',
  editLink: {
    baseUrl: 'https://github.com/nx-lang/nx/edit/main/docs'
  },
  sidebar: [
    {
      label: 'Overview',
      items: [
        { label: 'What is NX?', link: '/overview/what-is-nx' },
        { label: 'Design Goals', link: '/overview/design-goals' },
        { label: 'Comparison', link: '/overview/comparison' }
      ]
    },
    {
      label: 'Tutorials',
      items: [
        { label: 'Getting Started', link: '/tutorials/getting-started' },
        { label: 'Building Your First Component', link: '/tutorials/building-your-first-component' },
        { label: 'Working with Design Tokens', link: '/tutorials/working-with-design-tokens' }
      ]
    },
    {
      label: 'Reference',
      autogenerate: {
        directory: 'reference'
      }
    },
    {
      label: 'Contributing',
      items: [
        { label: 'Contributing Guide', link: '/contributing/index' }
      ]
    }
  ],
  expressiveCode: {
    themes: [darkPlus, lightPlus],
    useStarlightUiThemeColors: false,
    shiki: {
      langs: [nxLanguage]
    }
  }
};

export default config;
