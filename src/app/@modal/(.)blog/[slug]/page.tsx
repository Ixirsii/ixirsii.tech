"use client";

import { type ReactNode, use, useEffect, useState } from "react";
import { Modal } from "~/app/@modal/(.)blog/[slug]/modal";
import BlogPost from "~/app/_components/blog-post";
import { getBlogPost } from "~/server/db/queries";

export default function BlogModal({
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

  return (
    <Modal>
      <div className="overflow-y-scroll">
        <BlogPost blogPost={blogPost} />
      </div>
    </Modal>
  );
}
