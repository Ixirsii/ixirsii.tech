import Link from "next/link";
import type { ReactNode } from "react";

export function Header(): ReactNode {
  return (
    <header>
      <h1>
        <Link href="/">Ixirsii</Link>
      </h1>

      <nav>
        <Link className="nav-link" href="/about">
          About
        </Link>
        <Link className="nav-link" href="/blog">
          Blog
        </Link>
        <Link className="nav-link" href="/code">
          Code
        </Link>
      </nav>
    </header>
  );
}
