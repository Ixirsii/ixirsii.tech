import { type ReactNode } from "react";

import { Preview } from "~/app/blog/_components";
import { type BlogPostsWithTags, listBlogPosts } from "~/server/db";

export default async function Blog(): Promise<ReactNode> {
  const posts: BlogPostsWithTags = await listBlogPosts();

  return (
    <div className="grid grid-cols-2 gap-4">
      {posts.map((post) => (
        <Preview
          key={post.id}
          preview={post.preview ?? post.body}
          publishedAt={post.publishedAt}
          slug={post.slug}
          tags={post.postTags.map((postTag) => postTag.tag)}
          title={post.title}
        />
      ))}
    </div>
  );
}
