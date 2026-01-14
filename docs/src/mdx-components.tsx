import * as AccordionComponents from "fumadocs-ui/components/accordion";
import * as CalloutComponents from "fumadocs-ui/components/callout";
import { CodeBlock, Pre } from "fumadocs-ui/components/codeblock";
import * as StepComponents from "fumadocs-ui/components/steps";
import * as TabsComponents from "fumadocs-ui/components/tabs";
import defaultMdxComponents from "fumadocs-ui/mdx";
import type { MDXComponents } from "mdx/types";
import type { ComponentProps } from "react";
import { Algorithm } from "./components/algorithm";

export function getMDXComponents(components?: MDXComponents): MDXComponents {
    return {
        ...defaultMdxComponents,
        ...AccordionComponents,
        ...TabsComponents,
        ...StepComponents,
        ...CalloutComponents,
        ...components,
        Algorithm,
        pre: ({ children, ...props }: ComponentProps<typeof CodeBlock>) => (
            <CodeBlock {...props} viewportProps={{ className: "max-h-none" }}>
                <Pre>{children}</Pre>
            </CodeBlock>
        ),
    };
}
