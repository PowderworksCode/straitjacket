import type { MDXComponents } from "mdx/types";
import {
  getPowderworksMDXComponents,
  useMDXComponents as usePowderworksMDXComponents,
} from "@thepowderworks/fumadocs/mdx";

export function getMDXComponents(components?: MDXComponents) {
  return getPowderworksMDXComponents(components);
}

export const useMDXComponents = usePowderworksMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
