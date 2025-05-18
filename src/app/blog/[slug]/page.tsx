"use client";

import { type ReactNode, use, useEffect, useState } from "react";
import BlogPost from "~/app/_components/blog-post";
import { getBlogPost } from "~/server/db/queries";

export default function BlogPostPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}): ReactNode {
  const { slug } = use(params);
  const [blogPost, setBlogPost] = useState<Awaited<
    ReturnType<typeof getBlogPost>
  > | null>(null);

  useEffect(() => {
    getBlogPost(slug)
      .then(setBlogPost)
      .catch((error) => console.error(error));
  }, [slug]);

  return <BlogPost blogPost={blogPost} />;
}
