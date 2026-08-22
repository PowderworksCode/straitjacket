import { createPowderworksBaseOptions } from "@thepowderworks/fumadocs/layout";
import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";
import { site } from "./site";

export function baseOptions(
  links: BaseLayoutProps["links"] = [],
): BaseLayoutProps {
  return {
    ...createPowderworksBaseOptions(site, site.defaultLocale),
    links,
  };
}
