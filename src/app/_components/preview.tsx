import Link from "next/link";
import type { ReactNode } from "react";
import Markdown from "react-markdown";
import rehypeRaw from "rehype-raw";

import type { tags as tagsSchema } from "~/server/db";

export function Preview({
  preview,
  publishedAt,
  slug,
  tags,
  title,
  updatedAt,
}: Readonly<{
  preview: string;
  publishedAt: Date | null;
  slug: string;
  tags: (typeof tagsSchema.$inferSelect)[];
  title: string;
  updatedAt: Date | null;
}>): ReactNode {
  const publishedLocaleString = publishedAt?.toLocaleString();
  const updatedLocaleString = updatedAt?.toLocaleString();

  return (
    <Link className="preview" href={`/blog/${slug}`}>
      <h3>{title}</h3>

      <Markdown rehypePlugins={[rehypeRaw]}>{preview}</Markdown>

      <div className="preview-metadata">
        {publishedLocaleString !== null && (
          <>
            <div>Published at:</div>
            <div>{publishedLocaleString}</div>
          </>
        )}
        {updatedLocaleString !== null && updatedLocaleString !== publishedLocaleString && (
          <>
            <div>Updated at:</div>
            <div>{updatedLocaleString}</div>
          </>
        )}
        <div>Tags:</div>
        <div>{tags.map((tag) => tag.name).join(", ")}</div>
      </div>
    </Link>
  );
}
