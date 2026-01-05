"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";

export function Header(): ReactNode {
  const pathname = usePathname();

  return (
    <header>
      <h1>
        <Link href="/">Ixirsii</Link>
      </h1>

      <nav>
        <Link
          className={pathname.startsWith("/about") ? "active" : ""}
          href="/about"
        >
          About
        </Link>
        <Link
          className={pathname.startsWith("/blog") ? "active" : ""}
          href="/blog"
        >
          Blog
        </Link>
        <Link href="https://www.9dev.tech">Code</Link>
      </nav>
    </header>
  );
}
