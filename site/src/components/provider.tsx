"use client";

import { PowderworksProvider } from "@thepowderworks/fumadocs/provider";
import type { ReactNode } from "react";
import SearchDialog from "@/components/search";
import { translations } from "@/lib/i18n";

export function Provider({ children }: { children: ReactNode }) {
  return (
    <PowderworksProvider
      lang="en"
      translations={translations}
      search={{ SearchDialog }}
      theme={{ defaultTheme: "dark", enableSystem: false }}
    >
      {children}
    </PowderworksProvider>
  );
}
