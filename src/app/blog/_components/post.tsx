"use client";

import Link from "next/link";
import Shiitake from "shiitake";
import type * as schema from "~/server/db/schema";

export default function Post({
  createdAt,
  body,
  slug,
  tags,
  title,
  updatedAt,
}: {
  createdAt: Date,
  body: string,
  slug: string,
  tags: typeof schema.tags.$inferSelect[],
  title: string,
  updatedAt: Date,
}) {
  return (
    <Link
      className="bg-teal-800/10 hover:bg-teal-800/20 flex flex-col grow h-full p-4 rounded-xl"
      href={`/blog/${slug}`}
    >
      <h3>{title}</h3>
      <Shiitake lines={2} tagName="p">
        {body}
      </Shiitake>
      <div className="mt-auto text-sm">
        <div>{`Created at: ${createdAt.toLocaleString()}`}</div>
        <div>{`Updated at: ${updatedAt.toLocaleString()}`}</div>
        <div>{`Tags: ${tags.map(tag => tag.name).join(", ")}`}</div>
      </div>
    </Link>
  );
}
