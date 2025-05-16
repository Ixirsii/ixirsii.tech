import "~/styles/globals.css";

import { type Metadata } from "next";
import { Geist } from "next/font/google";
import Header from "~/app/_components/header";

export const metadata: Metadata = {
  title: "Ixirsii's blog",
  description: "Ixirsii's blog and development portfolio",
  icons: [{ rel: "icon", url: "/favicon.ico" }],
};

const geist = Geist({
  subsets: ["latin"],
  variable: "--font-geist-sans",
});

export default function RootLayout({
  children,
  modal,
}: Readonly<{ children: React.ReactNode, modal: React.ReactNode }>) {
  return (
    <html lang="en" className={`${geist.variable}`}>
      <body className="min-h-screen bg-slate-50">
        <Header />
        <main className="flex flex-col items-center justify-center">
          <div className="w-3/4">{children}</div>
        </main>
        {modal}
        <div id="modal-root" />
      </body>
    </html>
  );
}
