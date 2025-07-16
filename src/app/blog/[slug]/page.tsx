import { type ReactNode } from "react";

import { BlogPost } from "~/app/_components";
import { type BlogPostWithTags, getBlogPost } from "~/server/db";

export default async function BlogPostPage({
  params,
}: {
  params: Readonly<Promise<{ slug: string }>>;
}): Promise<ReactNode> {
  const { slug } = await params;
  const blogPost: BlogPostWithTags = await getBlogPost(slug);

  return <BlogPost blogPost={blogPost} />;
}
