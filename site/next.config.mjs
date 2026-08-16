// The turbopack root is pinned so the build does not resolve a workspace
// root from a lockfile in a parent directory.
import { fileURLToPath } from "node:url";
import { createMDX } from "fumadocs-mdx/next";

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  output: "export",
  reactStrictMode: true,
  turbopack: {
    root: fileURLToPath(new URL(".", import.meta.url)),
  },
};

export default withMDX(config);
