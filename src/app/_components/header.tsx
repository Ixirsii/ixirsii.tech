function Navigation({
  href,
  name
}: {
  href: string,
  name: string,
}) {
  return (
    <li className="inline-block list-none h-full w-1/3">
      <a className="hover:bg-white/25 inline-block h-full w-full" href={href}>
        {name}
      </a>
    </li>
  );
}

export default function Header() {
  return (
    <header className="bg-teal-800 h-24 justify-center text-white">
      <div className="h-16 text-center w-full">
        <a className="font-semibold leading-snug text-5xl" href="/">Ixirsii</a>
      </div>
      <div className="bg-teal-900 h-8 mx-[16.67%] w-2/3">
        <ul className="align-middle h-full text-center">
          <Navigation href="/about" name="About" />
          <Navigation href="/blog" name="Blog" />
          <Navigation href="/code" name="Code" />
        </ul>
      </div>
    </header>
  );
}
