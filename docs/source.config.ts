import {
    defineConfig,
    defineDocs,
    frontmatterSchema,
    metaSchema,
} from "fumadocs-mdx/config";
import rehypeKatex from "rehype-katex";
import rehypeStringify from "rehype-stringify";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";

// You can customise Zod schemas for frontmatter and `meta.json` here
// see https://fumadocs.dev/docs/mdx/collections
export const developers = defineDocs({
    dir: "content/developers",
    docs: {
        schema: frontmatterSchema,
        postprocess: {
            includeProcessedMarkdown: true,
        },
    },
    meta: {
        schema: metaSchema,
    },
});

export const protocol = defineDocs({
    dir: "content/protocol",
    docs: {
        schema: frontmatterSchema,
        postprocess: {
            includeProcessedMarkdown: true,
        },
    },
    meta: {
        schema: metaSchema,
    },
});

export default defineConfig({
    mdxOptions: {
        remarkPlugins: [
            remarkParse,
            remarkMath,
            // remarkRehype,
        ],
        rehypePlugins: (v) => [
            () =>
                rehypeKatex({
                    output: "html",
                    strict: "ignore",
                }),
            rehypeStringify,
            ...v,
        ],
    },
});
