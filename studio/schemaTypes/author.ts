import {defineField, defineType} from 'sanity'

export default defineType({
  name: 'author',
  title: 'Author',
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
    defineField({name: 'bio', title: 'Bio', type: 'text', rows: 4}),
    defineField({name: 'avatar', title: 'Avatar', type: 'image', options: {hotspot: true}}),
    defineField({name: 'avatarUrl', title: 'Legacy avatar URL', type: 'url'}),
    defineField({name: 'websiteUrl', title: 'Website URL', type: 'url'}),
    defineField({name: 'twitterHandle', title: 'X / Twitter handle', type: 'string'}),
    defineField({name: 'linkedinUrl', title: 'LinkedIn URL', type: 'url'}),
    defineField({name: 'facebookUrl', title: 'Facebook URL', type: 'url'}),
    defineField({name: 'instagramUrl', title: 'Instagram URL', type: 'url'}),
    defineField({name: 'whatsapp', title: 'WhatsApp', type: 'string'}),
    defineField({
      name: 'expertise',
      title: 'Expertise',
      type: 'array',
      of: [{type: 'string'}],
      options: {layout: 'tags'},
    }),
  ],
  preview: {
    select: {
      title: 'name',
      subtitle: 'slug.current',
      media: 'avatar',
    },
  },
})
