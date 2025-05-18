"use client";

import { type ReactNode, useEffect, useState } from "react";
import Post from "~/app/blog/_components/post";
import { listBlogPosts } from "~/server/db/queries";

export const dynamic = "force-dynamic";

export default function Blog(): ReactNode {
  const [posts, setPosts] = useState<Awaited<ReturnType<typeof listBlogPosts>>>(
    [],
  );

  useEffect(() => {
    listBlogPosts()
      .then(setPosts)
      .catch((error) => console.error(error));
  }, []);

  return (
    <>
      <div className="grid grid-cols-2 gap-4">
        {posts.map((post) => (
          <Post
            key={post.id}
            preview={post.preview ?? post.body}
            publishedAt={post.publishedAt}
            slug={post.slug}
            tags={post.postTags.map((postTag) => postTag.tag)}
            title={post.title}
          />
        ))}
      </div>
    </>
  );
}
