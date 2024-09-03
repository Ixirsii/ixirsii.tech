import NavLink from "./nav-link";

export default function Header() {
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
