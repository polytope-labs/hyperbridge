import { RootProvider } from "fumadocs-ui/provider/next";
import type { Metadata } from "next";
import "./global.css";

export const metadata: Metadata = {
    title: {
        template: "%s - Hyperbridge Documentation",
        default: "Hyperbridge Documentation",
    },
    openGraph: {
        images: "/og.png",
    },
};

export default function Layout({ children }: LayoutProps<"/">) {
    return (
        <html lang="en" suppressHydrationWarning>
            <body className="flex flex-col min-h-screen">
                <RootProvider
                    search={{
                        options: {
                            type: "static",
                        },
                    }}
                >
                    {children}
                </RootProvider>
            </body>
        </html>
    );
}
