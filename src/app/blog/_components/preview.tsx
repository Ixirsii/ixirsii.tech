import Link from "next/link";
import type { ReactNode } from "react";

import type { tags as tagsSchema } from "~/server/db";

export function Preview({
  preview,
  publishedAt,
  slug,
  tags,
  title,
}: Readonly<{
  preview: string;
  publishedAt: Date | null;
  slug: string;
  tags: (typeof tagsSchema.$inferSelect)[];
  title: string;
}>): ReactNode {
  return (
    <Link className="preview" href={`/blog/${slug}`}>
      <h3>{title}</h3>
      <p>{preview}</p>
      <div className="metadata">
        {publishedAt !== null && (
          <div>{`Published at: ${publishedAt.toLocaleString()}`}</div>
        )}
        <div>{`Tags: ${tags.map((tag) => tag.name).join(", ")}`}</div>
      </div>
    </Link>
  );
}
