import nxGrammar from '../../src/vscode/syntaxes/nx.tmLanguage.json' with { type: 'json' };

const nxLanguage = {
  ...nxGrammar,
  name: 'nx'
};

/** @type {import('@astrojs/starlight/types').StarlightUserConfig} */
const config = {
  title: 'NX',
  description:
    'NX is a typed language for markup, data and the logic between them, in one syntax that runs on .NET, in JavaScript and in the browser.',
  logo: {
    src: './src/assets/logo.svg'
  },
  favicon: '/favicon.svg',
  // The not-found page is src/pages/404.astro, which keeps it out of the docs collection.
  disable404Route: true,
  customCss: ['./src/styles/custom.css'],
  social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/nx-lang/nx' }],
  components: {
    Header: './src/components/Header.astro',
    MobileMenuFooter: './src/components/MobileMenuFooter.astro'
  },
  head: [
    { tag: 'meta', attrs: { property: 'og:image', content: 'https://nxlang.org/og.png' } },
    { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
    { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
    { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
    { tag: 'meta', attrs: { name: 'twitter:image', content: 'https://nxlang.org/og.png' } },
    // The commit the build came from, which the deploy workflow's smoke test looks for.
    { tag: 'meta', attrs: { name: 'nx-build', content: process.env.GITHUB_SHA ?? 'local' } }
  ],
  editLink: {
    baseUrl: 'https://github.com/nx-lang/nx/edit/main/sites/website'
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
      label: 'Language Tour',
      items: [
        { label: 'Elements', link: '/language-tour/elements' },
        { label: 'Functions & Bindings', link: '/language-tour/functions' },
        { label: 'Expressions & Control Flow', link: '/language-tour/expressions' },
        { label: 'Types', link: '/language-tour/types' },
        { label: 'Textual Content', link: '/language-tour/textual-content' },
        { label: 'Modules & Imports', link: '/language-tour/modules-and-imports' }
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
      items: [{ autogenerate: { directory: 'reference' } }]
    },
    {
      label: 'Contributing',
      items: [
        { label: 'Contributing Guide', link: '/contributing/index' }
      ]
    }
  ],
  expressiveCode: {
    themes: ['dark-plus', 'light-plus'],
    useStarlightUiThemeColors: false,
    shiki: {
      langs: [nxLanguage]
    }
  }
};

export default config;
