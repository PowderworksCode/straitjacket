import { HomeLayout } from "fumadocs-ui/layouts/home";
import { baseOptions } from "@/lib/layout.shared";

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <HomeLayout
      {...baseOptions([{ text: "Docs", url: "/docs", active: "nested-url" }])}
    >
      {children}
    </HomeLayout>
  );
}
