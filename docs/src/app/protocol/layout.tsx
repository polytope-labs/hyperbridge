import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { baseOptions } from "@/lib/layout.shared";
import { protocolSource } from "@/lib/source";

export default function Layout({ children }: LayoutProps<"/protocol">) {
    return (
        <DocsLayout tree={protocolSource.pageTree} {...baseOptions()}>
            {children}
        </DocsLayout>
    );
}
