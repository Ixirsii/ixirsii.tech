import { type Metadata } from "next";
import { type ReactNode } from "react";

import { Posts } from "~/app/_components";

export const dynamic = "force-dynamic";
export const metadata: Metadata = {
  title: "Home | Ixirsii",
};

export default function Home(): ReactNode {
  return <Posts />;
}
