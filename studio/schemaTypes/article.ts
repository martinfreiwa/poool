import {defineField, defineType} from 'sanity'

const statuses = [
  {title: 'Draft', value: 'draft'},
  {title: 'Published', value: 'published'},
  {title: 'Changes pending', value: 'changes_pending'},
  {title: 'Scheduled', value: 'scheduled'},
  {title: 'Taken down', value: 'taken_down'},
  {title: 'Archived', value: 'archived'},
]

const languages = [
  {title: 'English', value: 'en'},
  {title: 'Indonesian', value: 'id'},
  {title: 'German', value: 'de'},
  {title: 'Russian', value: 'ru'},
]

export default defineType({
  name: 'article',
  title: 'Article',
  type: 'document',
  groups: [
    {name: 'content', title: 'Content', default: true},
    {name: 'publishing', title: 'Publishing'},
    {name: 'seo', title: 'SEO'},
    {name: 'translations', title: 'Translations'},
  ],
  fields: [
    defineField({
      name: 'title',
      title: 'Title',
      type: 'string',
      group: 'content',
      validation: (rule) => rule.required(),
    }),
    defineField({
      name: 'slug',
      title: 'Slug',
      type: 'slug',
      group: 'content',
      options: {source: 'title', maxLength: 96},
      validation: (rule) => rule.required(),
    }),
    defineField({name: 'subtitle', title: 'Subtitle', type: 'string', group: 'content'}),
    defineField({
      name: 'excerpt',
      title: 'Excerpt',
      type: 'text',
      rows: 3,
      group: 'content',
      validation: (rule) => rule.required().max(300),
    }),
    defineField({
      name: 'body',
      title: 'Body',
      type: 'blockContent',
      group: 'content',
    }),
    defineField({
      name: 'bodyText',
      title: 'Plain-text body fallback',
      type: 'text',
      rows: 8,
      group: 'content',
      description: 'Used by legacy imports and the POOOL admin editor.',
    }),
    defineField({
      name: 'coverImage',
      title: 'Cover image',
      type: 'image',
      options: {hotspot: true},
      group: 'content',
      fields: [
        defineField({name: 'alt', title: 'Alternative text', type: 'string'}),
      ],
    }),
    defineField({name: 'coverImageUrl', title: 'Legacy cover image URL', type: 'url', group: 'content'}),
    defineField({
      name: 'author',
      title: 'Author',
      type: 'reference',
      to: [{type: 'author'}],
      group: 'content',
      validation: (rule) => rule.required(),
    }),
    defineField({
      name: 'category',
      title: 'Category',
      type: 'reference',
      to: [{type: 'category'}],
      group: 'content',
      validation: (rule) => rule.required(),
    }),
    defineField({
      name: 'tags',
      title: 'Tags',
      type: 'array',
      of: [{type: 'string'}],
      options: {layout: 'tags'},
      group: 'content',
    }),
    defineField({name: 'featured', title: 'Featured', type: 'boolean', initialValue: false, group: 'publishing'}),
    defineField({
      name: 'shareLinks',
      title: 'Article share links',
      type: 'object',
      group: 'publishing',
      description: 'Optional overrides for the social buttons shown on this public article.',
      fields: [
        defineField({name: 'whatsappUrl', title: 'WhatsApp URL', type: 'url'}),
        defineField({name: 'facebookUrl', title: 'Facebook URL', type: 'url'}),
        defineField({name: 'xUrl', title: 'X URL', type: 'url'}),
        defineField({name: 'instagramUrl', title: 'Instagram URL', type: 'url'}),
        defineField({name: 'linkedinUrl', title: 'LinkedIn URL', type: 'url'}),
      ],
    }),
    defineField({
      name: 'status',
      title: 'Status',
      type: 'string',
      options: {list: statuses, layout: 'radio'},
      initialValue: 'draft',
      group: 'publishing',
      validation: (rule) => rule.required(),
    }),
    defineField({name: 'publishedAt', title: 'Published at', type: 'datetime', group: 'publishing'}),
    defineField({name: 'scheduledAt', title: 'Scheduled at', type: 'datetime', group: 'publishing'}),
    defineField({name: 'readingTimeMinutes', title: 'Reading time minutes', type: 'number', initialValue: 5, group: 'publishing'}),
    defineField({
      name: 'language',
      title: 'Language',
      type: 'string',
      options: {list: languages},
      group: 'translations',
      description: 'Used by the POOOL admin translation-readiness badges.',
    }),
    defineField({
      name: 'translations',
      title: 'Translations',
      type: 'array',
      of: [{type: 'reference', to: [{type: 'article'}], weak: true}],
      group: 'translations',
      description: 'Link Indonesian, German, and Russian article variants here.',
    }),
    defineField({name: 'metaTitle', title: 'SEO title', type: 'string', group: 'seo'}),
    defineField({name: 'metaDescription', title: 'SEO description', type: 'text', rows: 3, group: 'seo'}),
    defineField({name: 'canonicalUrl', title: 'Canonical URL', type: 'url', group: 'seo'}),
    defineField({name: 'ogImage', title: 'Open Graph image', type: 'image', options: {hotspot: true}, group: 'seo'}),
    defineField({
      name: 'schemaType',
      title: 'Schema type',
      type: 'string',
      initialValue: 'BlogPosting',
      group: 'seo',
    }),
  ],
  orderings: [
    {
      title: 'Published date, newest first',
      name: 'publishedAtDesc',
      by: [{field: 'publishedAt', direction: 'desc'}],
    },
  ],
  preview: {
    select: {
      title: 'title',
      author: 'author.name',
      status: 'status',
      media: 'coverImage',
    },
    prepare(selection) {
      const {title, author, status, media} = selection
      return {
        title,
        subtitle: [author, status].filter(Boolean).join(' · '),
        media,
      }
    },
  },
})
