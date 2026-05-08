import { Analytics } from "@vercel/analytics/next";
import { SpeedInsights } from "@vercel/speed-insights/next";
import { type Metadata } from "next";
import { type ReactNode } from "react";

import { Header } from "~/app/_components";
import "~/styles/globals.css";

export const metadata: Metadata = {
  description: "Ixirsii's blog",
  icons: [{ rel: "icon", url: "/favicon.ico" }],
};

export default function RootLayout({
  children,
}: Readonly<{ children: ReactNode }>): ReactNode {
  return (
    <html lang="en">
      <body>
        <Header />

        <main>{children}</main>

        <Analytics />
        <SpeedInsights />
      </body>
    </html>
  );
}
