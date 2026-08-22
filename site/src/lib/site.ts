import { definePowderworksSite } from "@thepowderworks/fumadocs/config";

export const site = definePowderworksSite({
  name: "Straitjacket",
  description: "A secret scanner, but for slop.",
  repository: "PowderworksCode/straitjacket",
  siteUrl: "https://straitjacket.dev",
  mark: { src: "/strait-face.png", alt: "Straitjacket" },
  locales: [{ code: "en", name: "English", searchLanguage: "english" }],
  defaultLocale: "en",
});
