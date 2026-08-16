// Static Orama search index. The language list is at
// https://docs.orama.com/docs/orama-js/supported-languages
import { createFromSource } from "fumadocs-core/search/server";
import { source } from "@/lib/source";

export const revalidate = false;

export const { staticGET: GET } = createFromSource(source, {
  language: "english",
});
