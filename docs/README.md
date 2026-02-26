# new-docs

This is a Next.js application generated with
[Create Fumadocs](https://github.com/fuma-nama/fumadocs).

Run development server:

```bash
npm run dev
# or
pnpm dev
# or
yarn dev
```

Open http://localhost:3000 with your browser to see the result.

## Explore

In the project, you can see:

- `lib/source.ts`: Code for content source adapter, [`loader()`](https://fumadocs.dev/docs/headless/source-api) provides the interface to access your content.
- `lib/layout.shared.tsx`: Shared options for layouts, optional but preferred to keep.

| Route                     | Description                                            |
| ------------------------- | ------------------------------------------------------ |
| `app/(home)`              | The route group for your landing page and other pages. |
| `app/docs/developers`     | The documentation layout and pages.                    |
| `app/docs/ui`             | The documentation the Frontend integrations            |
| `app/api/search/route.ts` | The Route Handler for search.                          |

### Fumadocs MDX

A `source.config.ts` config file has been included, you can customise different options like frontmatter schema.

Read the [Introduction](https://fumadocs.dev/docs/mdx) for further details.

## SEO & Sitemap

The project is configured to automatically generate a sitemap on build using [next-sitemap](https://github.com/iamvishnusankar/next-sitemap).

- **Configuration**: `next-sitemap.config.js` (configured with `outDir: "./out"`)
- **Build Command**: `pnpm build` (runs `next build && next-sitemap`)
- **Output**: Generated directly to the build output directory
  - `out/sitemap.xml` - Main sitemap with all pages
  - `out/robots.txt` - Search engine crawler instructions

The sitemap is automatically updated every time you run the build command and includes all static routes from the documentation. Note: The sitemap is generated directly to `out/`, not to `public/`, as this is a static export build.

## Learn More
</text>


To learn more about Next.js and Fumadocs, take a look at the following
resources:

- [Next.js Documentation](https://nextjs.org/docs) - learn about Next.js
  features and API.
- [Learn Next.js](https://nextjs.org/learn) - an interactive Next.js tutorial.
- [Fumadocs](https://fumadocs.dev) - learn about Fumadocs
