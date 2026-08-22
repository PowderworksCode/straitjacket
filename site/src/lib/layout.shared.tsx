import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";
import { appName, gitConfig } from "./shared";

export function baseOptions(
  links: BaseLayoutProps["links"] = [],
): BaseLayoutProps {
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
    links,
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
