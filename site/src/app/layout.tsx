import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Provider } from "@/components/provider";
import "./global.css";

const inter = Inter({
  subsets: ["latin"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://straitjacket.dev"),
  title: {
    default: "Straitjacket — a secret scanner, but for slop",
    template: "%s — Straitjacket",
  },
  description:
    "Straitjacket is a fast, deterministic scanner that flags the weird code and text LLMs tend to generate. One static Rust binary, drops into any CI.",
};

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={inter.className} suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
