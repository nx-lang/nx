import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Config } from '@docusaurus/types';
import type { ThemeConfig } from '@docusaurus/preset-classic';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const nxGrammarPath = path.resolve(__dirname, '../src/vscode/syntaxes/nx.tmLanguage.json');
const nxGrammar = JSON.parse(fs.readFileSync(nxGrammarPath, 'utf-8')) as Record<string, unknown>;

const nxScopeName = typeof nxGrammar.scopeName === 'string' ? nxGrammar.scopeName : 'source.nx';

const config: Config = {
  title: 'NX Language',
  tagline: 'Official documentation for the NX language.',
  url: 'https://nx-lang.dev',
  baseUrl: '/',
  organizationName: 'nx-lang',
  projectName: 'nx',
  onBrokenLinks: 'throw',
  i18n: {
    defaultLocale: 'en',
    locales: ['en']
  },
  presets: [
    [
      'classic',
      {
        docs: {
          path: path.resolve(__dirname, '../docs'),
          routeBasePath: '/',
          sidebarPath: path.resolve(__dirname, './sidebars.ts'),
          editUrl: 'https://github.com/nx-lang/nx/tree/main/docs/',
          showLastUpdateAuthor: true,
          showLastUpdateTime: true
        },
        blog: false,
        pages: false,
        theme: {
          customCss: path.resolve(__dirname, './src/css/custom.css')
        }
      }
    ]
  ],
  markdown: {
    format: 'mdx',
    hooks: {
      onBrokenMarkdownLinks: 'warn'
    }
  },
  themeConfig: {
    navbar: {
      title: 'NX Docs',
      items: [
        {
          to: '/',
          label: 'Docs',
          position: 'left'
        },
        {
          href: 'https://github.com/nx-lang/nx',
          label: 'GitHub',
          position: 'right'
        }
      ]
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Overview',
              to: '/'
            },
            {
              label: 'Tutorials',
              to: '/tutorials/getting-started'
            },
            {
              label: 'Reference',
              to: '/reference/syntax/if'
            }
          ]
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub Issues',
              href: 'https://github.com/nx-lang/nx/issues'
            }
          ]
        }
      ],
      copyright: `Copyright © ${new Date().getFullYear()} NX.`
    }
  } satisfies ThemeConfig
};

export default config;
