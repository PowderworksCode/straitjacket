import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";
import { appName, gitConfig } from "./shared";

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="inline-flex items-center gap-2">
          <img
            src="/strait-face.png"
            alt=""
            aria-hidden
            width={22}
            height={22}
            className="rounded-sm"
          />
          {appName}
        </span>
      ),
    },
    links: [
      { text: "Tutorials", url: "/docs/tutorials", active: "nested-url" },
      { text: "Guides", url: "/docs/guides", active: "nested-url" },
      { text: "Reference", url: "/docs/reference", active: "nested-url" },
      { text: "Explanation", url: "/docs/explanation", active: "nested-url" },
      { text: "About", url: "/docs/about", active: "nested-url" },
    ],
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
