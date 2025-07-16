import "~/styles/globals.css";

import type { Metadata } from "next";
import type { ReactNode } from "react";

import { Header } from "~/app/_components";

export const metadata: Metadata = {
  title: "Ixirsii's blog",
  description: "Ixirsii's blog and development portfolio",
  icons: [{ rel: "icon", url: "/favicon.ico" }],
};

export default function RootLayout({
  children,
  modal,
}: Readonly<{ children: ReactNode; modal: ReactNode }>): ReactNode {
  return (
    <html lang="en">
      <body>
        <Header />

        <main>{children}</main>
        {modal}
        <div id="modal-root" />
      </body>
    </html>
  );
}
