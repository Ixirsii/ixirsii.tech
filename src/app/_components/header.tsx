import Link from "next/link";
import NavLink from "~/app/_components/nav-link";

export default function Header() {
  return (
    <header className="h-32 justify-center bg-teal-800 text-white">
      <h1 className="mb-0 h-24 text-center text-7xl leading-snug font-bold text-white">
        <Link href="/">Ixirsii</Link>
      </h1>
      <nav className="mx-[12.5%] flex h-8 w-3/4 justify-between bg-teal-900 font-semibold">
        <NavLink href="/about" name="About" />
        <NavLink href="/blog" name="Blog" />
        <NavLink href="/code" name="Code" />
      </nav>
    </header>
  );
}
