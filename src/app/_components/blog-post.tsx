import type { ReactNode } from "react";
import Markdown from "react-markdown";

import { type BlogPostWithTags } from "~/server/db";

export function BlogPost({
  blogPost,
}: Readonly<{ blogPost: BlogPostWithTags }>): ReactNode {
  if (!blogPost) {
    return null;
  }

  return (
    <>
      <Markdown>{blogPost.body}</Markdown>
      <div className="text-sm">
        {blogPost.publishedAt !== null && (
          <div>{`Published at: ${blogPost.publishedAt?.toLocaleString()}`}</div>
        )}
        {blogPost.updatedAt !== null && (
          <div>{`Updated at: ${blogPost.updatedAt?.toLocaleString()}`}</div>
        )}
      </div>
    </>
  );
}
