import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Overview',
      collapsed: false,
      items: [
        'overview/what-is-nx',
        'overview/design-goals',
        'overview/comparison'
      ]
    },
    {
      type: 'category',
      label: 'Tutorials',
      items: [
        'tutorials/getting-started',
        'tutorials/building-your-first-component',
        'tutorials/working-with-design-tokens'
      ]
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        {
          type: 'category',
          label: 'Syntax',
          items: [
            'reference/syntax/if',
            'reference/syntax/for',
            'reference/syntax/expressions'
          ]
        }
      ]
    },
    'contributing/index'
  ]
};

export default sidebars;
