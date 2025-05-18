"use client";

import Link from "next/link";
import type * as schema from "~/server/db/schema";

export default function Post({
  preview,
  publishedAt,
  slug,
  tags,
  title,
}: {
  preview: string;
  publishedAt: Date | null;
  slug: string;
  tags: (typeof schema.tags.$inferSelect)[];
  title: string;
}) {
  return (
    <Link
      className="flex h-full grow flex-col rounded-xl bg-teal-800/10 p-4 hover:bg-teal-800/20"
      href={`/blog/${slug}`}
    >
      <h3>{title}</h3>
      <p>{preview}</p>
      <div className="mt-auto text-sm">
        {publishedAt !== null && (
          <div>{`Published at: ${publishedAt.toLocaleString()}`}</div>
        )}
        <div>{`Tags: ${tags.map((tag) => tag.name).join(", ")}`}</div>
      </div>
    </Link>
  );
}
