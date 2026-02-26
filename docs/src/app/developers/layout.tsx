import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { baseOptions } from "@/lib/layout.shared";
import { backendSource } from "@/lib/source";

export default function Layout({ children }: LayoutProps<"/developers">) {
    return (
        <DocsLayout tree={backendSource.pageTree} {...baseOptions()}>
            {children}
        </DocsLayout>
    );
}
