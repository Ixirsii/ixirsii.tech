import type { ReactNode } from "react";
import Markdown from "react-markdown";

import { type BlogPostWithTags } from "~/server/db";

export function Post({
  post,
}: Readonly<{ post: BlogPostWithTags }>): ReactNode {
  if (!post) {
    return null;
  }

  return (
    <>
      <h1>{post.title}</h1>

      <div className="post-metadata">
        {post.publishedAt !== null && (
          <div>{`Published at: ${post.publishedAt.toLocaleString()}`}</div>
        )}
        {post.updatedAt !== null && (
          <div>{`Updated at: ${post.updatedAt.toLocaleString()}`}</div>
        )}
        <div>{`Tags: ${post.postTags.map((tag) => tag.tag.name).join(", ")}`}</div>
      </div>

      <hr />

      <Markdown>{post.body}</Markdown>
    </>
  );
}
