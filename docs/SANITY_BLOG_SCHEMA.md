# Sanity Blog Schema

The backend expects these public Sanity document types in project `3y7eud93`, dataset `production`.

## `author`

- `name`: string
- `slug`: slug
- `bio`: text
- `avatar`: image
- `websiteUrl`: url
- `twitterHandle`: string
- `linkedinUrl`: url
- `facebookUrl`: url
- `instagramUrl`: url
- `whatsapp`: string
- `expertise`: array of string

## `category`

- `name`: string
- `slug`: slug
- `description`: text
- `color`: string
- `icon`: string
- `sortOrder`: number
- `metaTitle`: string
- `metaDescription`: text

## `article`

- `title`: string
- `slug`: slug
- `subtitle`: string
- `excerpt`: text
- `body`: block content / Portable Text
- `coverImage`: image
- `author`: reference to `author`
- `category`: reference to `category`
- `tags`: array of string
- `featured`: boolean
- `publishedAt`: datetime
- `readingTimeMinutes`: number
- `metaTitle`: string
- `metaDescription`: text
- `canonicalUrl`: url
- `ogImage`: image
- `schemaType`: string, default `BlogPosting`
- `faqData`: object

Published reads use the public Sanity CDN and exclude drafts. No API token is required for public blog rendering.
