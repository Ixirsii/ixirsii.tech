import "~/styles/globals.css";

import { GeistSans } from "geist/font/sans";

import { TRPCReactProvider } from "~/trpc/react";
import Link from "next/link";

function NavLink({
  href,
  name,
}: {
  href: string,
  name: string,
}) {
  return (
    <Link className="hover:bg-white/25 leading-8 text-center w-1/3" href={href}>
      {name}
    </Link>
  );
}

function Header() {
  return (
    <header className="bg-teal-800 h-32 justify-center text-white">
      <h1 className="font-bold h-24 leading-snug mb-0 text-center text-white text-7xl">
        <a  href="/">Ixirsii</a>
      </h1>
      <nav className="bg-teal-900 flex font-semibold h-8 justify-between mx-[12.5%] w-3/4">
        <NavLink href="/about" name="About" />
        <NavLink href="/blog" name="Blog" />
        <NavLink href="/code" name= "Code" />
      </nav>
    </header>
  );
}

export const metadata = {
  title: "Ixirsii's blog",
  description: "Ixirsii's blog and development portfolio",
  icons: [{ rel: "icon", url: "/favicon.ico" }],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
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
      </body>
    </html>
  );
}
