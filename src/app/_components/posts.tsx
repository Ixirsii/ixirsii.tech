import { type ReactNode } from "react";

import { Preview } from "~/app/_components";
import { type BlogPostsWithTags, listBlogPosts } from "~/server/db";

export async function Posts(): Promise<ReactNode> {
  const posts: BlogPostsWithTags = await listBlogPosts();

  return (
    <div className="posts">
      {posts.map((post) => (
        <Preview
          key={post.id}
          preview={post.preview ?? post.body}
          publishedAt={post.publishedAt}
          slug={post.slug}
          tags={post.postTags.map((postTag) => postTag.tag)}
          title={post.title}
          updatedAt={post.updatedAt}
        />
      ))}
    </div>
  );
}

