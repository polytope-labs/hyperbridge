import path from "path";
import fs from "fs-extra";
import { glob } from "glob";

// /myPath/index.html => /myPath.html
(async function FixTrailingSlash(outDir = "./dist") {
  const pattern = path.join(outDir, "/**/index.html");
  const filePaths = (await glob(pattern)).filter((filePath) => {
    return filePath !== path.join(outDir, "/index.html");
  });

  await Promise.all(
    filePaths.map(async (filePath) => {
      if ((await fs.stat(filePath)).isDirectory()) {
        return;
      }
      const filePathCopy = `${path.dirname(filePath)}.html`;
      if (await fs.pathExists(filePathCopy)) {
      } else {
        await fs.copyFile(filePath, filePathCopy);
        await fs.rm(filePath);
      }
    }),
  );
})();
