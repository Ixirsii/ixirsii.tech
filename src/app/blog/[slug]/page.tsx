import { type Metadata } from "next";
import { type ReactNode } from "react";

import { Post } from "~/app/_components";
import { type BlogPostWithTags, getBlogPost } from "~/server/db";

type Props = Readonly<{ params: Promise<{ slug: string }> }>;

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  const post: BlogPostWithTags = await getBlogPost(slug);

  if (!post) {
    return {
      title: "Not Found | Ixirsii",
    };
  }

  return {
    title: `${post.title} | Ixirsii`,
  };
}

export default async function BlogPostPage({
  params,
}: {
  params: Readonly<Promise<{ slug: string }>>;
}): Promise<ReactNode> {
  const { slug } = await params;
  const post: BlogPostWithTags = await getBlogPost(slug);

  return <Post post={post} />;
}
