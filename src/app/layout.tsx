import "~/styles/globals.css";

import { GeistSans } from "geist/font/sans";

import Header from "~/app/_components/header";
import { TRPCReactProvider } from "~/trpc/react";

export const metadata = {
  title: "Ixirsii's blog",
  description: "Ixirsii's blog and development portfolio",
  icons: [{ rel: "icon", url: "/favicon.ico" }],
};

export default function RootLayout({
  children,
  modal,
}: {
  children: React.ReactNode;
  modal: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${GeistSans.variable}`}>
      <body className="min-h-screen bg-slate-50">
        <Header />
        <main className="flex flex-col items-center justify-center">
          <div className="w-3/4">
            <TRPCReactProvider>{children}</TRPCReactProvider>
          </div>
        </main>
        {modal}
        <div id="modal-root" />
      </body>
    </html>
  );
}
