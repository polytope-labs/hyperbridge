import { DocsBody, DocsPage } from "fumadocs-ui/layouts/docs/page";
import { createRelativeLink } from "fumadocs-ui/mdx";
import { notFound } from "next/navigation";
import type { Metadata } from "next/types";
import { backendSource, getPageImage } from "@/lib/source";
import { getMDXComponents } from "@/mdx-components";

export default async function Page(
    props: PageProps<"/developers/[[...slug]]">,
) {
    const params = await props.params;
    const page = backendSource.getPage(params.slug);
    if (!page) notFound();

    const MDX = page.data.body;

    return (
        <DocsPage toc={page.data.toc} full={page.data.full}>
            {/*<DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>*/}
            <DocsBody>
                <MDX
                    components={getMDXComponents({
                        // this allows you to link to other pages with relative file paths
                        a: createRelativeLink(backendSource, page),
                    })}
                />
            </DocsBody>
        </DocsPage>
    );
}

export async function generateStaticParams() {
    return backendSource.generateParams();
}

export async function generateMetadata(
    props: PageProps<"/developers/[[...slug]]">,
): Promise<Metadata> {
    const params = await props.params;
    const page = backendSource.getPage(params.slug);
    if (!page) notFound();

    return {
        title: page.data.title,
        description: page.data.description,
        openGraph: {
            images: getPageImage(page).url,
        },
    };
}
