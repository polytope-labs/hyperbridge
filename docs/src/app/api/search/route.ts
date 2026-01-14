import { createFromSource } from "fumadocs-core/search/server";
import { combinedSource } from "@/lib/source";

export const revalidate = false;

export const { staticGET: GET } = createFromSource(combinedSource, {
    language: "english",
});
