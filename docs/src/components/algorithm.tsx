// @ts-expect-error No type definition for this package
import { renderToString } from "pseudocode";

export function Algorithm({
  content,
  options = RENDER_OPTIONS,
}: {
  content: string;
  options?: RenderOptions;
}) {
  return (
    <div
      // biome-ignore lint/security/noDangerouslySetInnerHtmlWithChildren: SSR
      // biome-ignore lint/security/noDangerouslySetInnerHtml: SSR
      dangerouslySetInnerHTML={{
        __html: renderToString(content, { ...RENDER_OPTIONS, ...options }),
      }}
    ></div>
  );
}

const RENDER_OPTIONS: RenderOptions = {
  indentSize: "1.5em",
  commentDelimiter: "//",
  lineNumber: true,
  lineNumberPunc: ":",
  noEnd: false,
  captionCount: undefined,
  titlePrefix: "Algorithm ",
};

type RenderOptions = {
  indentSize: `${string}em`;
  commentDelimiter: "//";
  lineNumber: boolean;
  lineNumberPunc: string;
  noEnd: boolean;
  captionCount?: string;
  titlePrefix: string;
};
