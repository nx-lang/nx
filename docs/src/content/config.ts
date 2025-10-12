import { defineCollection } from 'astro:content';
import { docsSchema } from '@astrojs/starlight/schema';

const docs = defineCollection({
  type: 'content',
  schema: docsSchema()
});

export const collections = { docs };
