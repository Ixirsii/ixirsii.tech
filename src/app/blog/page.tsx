import { type Metadata } from "next";
import { type ReactNode } from "react";

import { Posts } from "~/app/_components";

export const dynamic = "force-dynamic";
export const metadata: Metadata = {
  title: "Blog | Ixirsii",
};

export default async function Blog(): Promise<ReactNode> {
  return <Posts />;
}
