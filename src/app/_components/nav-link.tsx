import Link from "next/link";

export default function NavLink({
  href,
  name,
}: {
  href: string;
  name: string;
}) {
  return (
    <Link className="w-1/3 text-center leading-8 hover:bg-white/25" href={href}>
      {name}
    </Link>
  );
}
