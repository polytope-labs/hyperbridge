import { developers, protocol } from "fumadocs-mdx:collections/server";
import { type InferPageType, loader } from "fumadocs-core/source";
import { lucideIconsPlugin } from "fumadocs-core/source/lucide-icons";

// See https://fumadocs.dev/docs/headless/source-api for more info
export const backendSource = loader({
    baseUrl: "/developers",
    source: developers.toFumadocsSource(),
    plugins: [lucideIconsPlugin()],
});

export const protocolSource = loader({
    baseUrl: "/protocol",
    source: protocol.toFumadocsSource(),
    plugins: [lucideIconsPlugin()],
});

export function getPageImage(page: InferPageType<typeof backendSource>) {
    const segments = [...page.slugs, "image.png"];

    return {
        segments,
        url: `/og/developers/${segments.join("/")}`,
    };
}

export function getPageProtocolImage(
    page: InferPageType<typeof protocolSource>,
) {
    const segments = [...page.slugs, "image.png"];

    return {
        segments,
        url: `/og/protocol/${segments.join("/")}`,
    };
}

export async function getLLMText(page: InferPageType<typeof backendSource>) {
    const processed = await page.data.getText("processed");

    return `# ${page.data.title}

${processed}`;
}

// Combined source for search indexing
// Preserve correct URL paths by prefixing with section names
const developersFiles = developers.toFumadocsSource();
const protocolFiles = protocol.toFumadocsSource();

export const combinedSource = loader({
    baseUrl: "/",
    source: {
        files: [
            ...developersFiles.files.map((file) => ({
                ...file,
                path: `developers/${file.path}`,
            })),
            ...protocolFiles.files.map((file) => ({
                ...file,
                path: `protocol/${file.path}`,
            })),
        ],
    },
    plugins: [lucideIconsPlugin()],
});
