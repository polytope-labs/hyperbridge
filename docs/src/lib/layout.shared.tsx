import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";
import Image from "next/image";
import logoBlack from "@/assets/logo_black.svg";
import logoWhite from "@/assets/logo_white.svg";

export function baseOptions(): BaseLayoutProps {
    return {
        githubUrl: "https://github.com/polytope-labs/hyperbridge",
        themeSwitch: {
            enabled: true,
        },
        nav: {
            title: (
                <div className="px-2">
                    <Image
                        src={logoBlack}
                        alt="Hyperbridge Logo"
                        width={149}
                        height={32}
                        className="min-h-8 dark:hidden"
                    />
                    <Image
                        src={logoWhite}
                        alt="Hyperbridge Logo"
                        width={149}
                        height={32}
                        className="min-h-8 hidden dark:block"
                    />
                    <span className="sr-only">Hyperbridge Docs</span>
                </div>
            ),
            transparentMode: "top",
        },
        links: [
            // { url: "", icon: <span>I</span>, label: "Nothing", text: "Some text" }
        ],
    };
}
