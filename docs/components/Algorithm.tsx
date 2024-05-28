import * as React from "react";

export default function Algorithm({
  content,
  algID,
  options = {
    indentSize: "1.5em",
    commentDelimiter: "//",
    lineNumber: true,
    lineNumberPunc: ":",
    noEnd: false,
    captionCount: undefined,
    titlePrefix: "Algorithm ",
  },
}: any) {
  const onLoad = () => {
    // Call pseudocode.renderElement() after both KaTeX and pseudocode libraries are loaded
    // also save the anchor element to scroll to it later, because pseudocode.renderElement() will change the page height
    var anchorElement = document.getElementById(
      window.location.hash.substring(1),
    );

    const elememt = document.getElementById(`_ps_${algID}`);
    if (!!(window as any).pseudocode && !!elememt) {
      (window as any).pseudocode.renderElement(
        elememt,
        options,
      );
    }

    if (anchorElement) {
      anchorElement.scrollIntoView();
    }
  };

  React.useEffect(() => {
    if (window && document) {
      const katexScript = document.createElement("script");
      katexScript.src =
        "https://cdn.jsdelivr.net/npm/katex@latest/dist/katex.min.js";
      katexScript.addEventListener("load", () => {
        // Load KaTeX library dynamically
        let pseudocodeScript = document.getElementById(
          "pseudocode-script",
        ) as HTMLScriptElement;
        if (!pseudocodeScript) {
          // Load pseudocode rendering library dynamically
          pseudocodeScript = document.createElement("script");
          pseudocodeScript.id = "pseudocode-script";
          pseudocodeScript.src =
            "https://cdn.jsdelivr.net/npm/pseudocode@latest/build/pseudocode.min.js";

          pseudocodeScript.addEventListener("load", onLoad);
        } else {
          onLoad()
        }

        document.body.appendChild(pseudocodeScript);
      });
      document.body.appendChild(katexScript);
    }
  }, []);

  const openingTag = `<pre class="scopeline-pseudocode" id="_ps_${algID}" style="display: hidden" >`;
  const closingTag = `</pre>`;
  return (
    <div
      dangerouslySetInnerHTML={{ __html: openingTag + content + closingTag }}
    />
  );
}
