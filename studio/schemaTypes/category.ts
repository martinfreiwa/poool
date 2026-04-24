import {defineField, defineType} from 'sanity'

export default defineType({
  name: 'category',
  title: 'Category',
  type: 'document',
  fields: [
    defineField({
      name: 'name',
      title: 'Name',
      type: 'string',
      validation: (rule) => rule.required(),
    }),
    defineField({
      name: 'slug',
      title: 'Slug',
      type: 'slug',
      options: {source: 'name', maxLength: 96},
      validation: (rule) => rule.required(),
    }),
    defineField({name: 'description', title: 'Description', type: 'text', rows: 3}),
    defineField({
      name: 'color',
      title: 'Color',
      type: 'string',
      description: 'Hex color used by the POOOL admin and blog UI.',
    }),
    defineField({name: 'icon', title: 'Icon', type: 'string'}),
    defineField({name: 'sortOrder', title: 'Sort order', type: 'number', initialValue: 0}),
    defineField({name: 'metaTitle', title: 'SEO title', type: 'string'}),
    defineField({name: 'metaDescription', title: 'SEO description', type: 'text', rows: 3}),
  ],
  orderings: [
    {
      title: 'Sort order',
      name: 'sortOrderAsc',
      by: [{field: 'sortOrder', direction: 'asc'}],
    },
  ],
  preview: {
    select: {
      title: 'name',
      subtitle: 'slug.current',
    },
  },
})
