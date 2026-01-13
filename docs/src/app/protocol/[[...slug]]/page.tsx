import { DocsBody, DocsPage } from "fumadocs-ui/layouts/docs/page";
import { createRelativeLink } from "fumadocs-ui/mdx";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getPageProtocolImage, protocolSource } from "@/lib/source";
import { getMDXComponents } from "@/mdx-components";

export default async function Page(props: PageProps<"/protocol/[[...slug]]">) {
    const params = await props.params;
    const page = protocolSource.getPage(params.slug);
    if (!page) notFound();

    const MDX = page.data.body;

    return (
        <DocsPage toc={page.data.toc} full={page.data.full}>
            <DocsBody>
                <MDX
                    components={getMDXComponents({
                        // this allows you to link to other pages with relative file paths
                        a: createRelativeLink(protocolSource, page),
                    })}
                />
            </DocsBody>
        </DocsPage>
    );
}

export async function generateStaticParams() {
    return protocolSource.generateParams();
}

export async function generateMetadata(
    props: PageProps<"/protocol/[[...slug]]">,
): Promise<Metadata> {
    const params = await props.params;
    const page = protocolSource.getPage(params.slug);
    if (!page) notFound();

    return {
        title: page.data.title,
        description: page.data.description,
        openGraph: {
            images: getPageProtocolImage(page).url,
        },
    };
}
