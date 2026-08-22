import type { MetadataRoute } from "next";
import { source } from "@/lib/source";

const BASE = "https://straitjacket.dev";

export const dynamic = "force-static";

export default function sitemap(): MetadataRoute.Sitemap {
  const staticRoutes = ["", "/about", "/contact", "/privacy"].map((path) => ({
    url: `${BASE}${path}`,
    lastModified: new Date(),
  }));

  const docsRoutes = source.getPages().map((page) => ({
    url: `${BASE}${page.url}`,
    lastModified: new Date(),
  }));

  return [...staticRoutes, ...docsRoutes];
}
