import { type ReactNode } from "react";

import { Modal } from "~/app/@modal/_components";
import { BlogPost } from "~/app/_components";
import { type BlogPostWithTags, getBlogPost } from "~/server/db";

export default async function BlogModal({
  params,
}: {
  params: Readonly<Promise<{ slug: string }>>;
}): Promise<ReactNode> {
  const { slug } = await params;
  const blogPost: BlogPostWithTags = await getBlogPost(slug);

  return (
    <Modal>
      <BlogPost blogPost={blogPost} />
    </Modal>
  );
}
