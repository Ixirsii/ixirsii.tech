import "server-only";
import { db } from "~/server/db";

export async function getBlogPost(slug: string) {
  return await db.query.posts.findFirst({
    where: (post, { and, eq }) =>
      and(eq(post.slug, slug), eq(post.isPublished, true)),
    with: {
      postTags: {
        with: {
          tag: true,
        },
      },
    },
  });
}

export async function listBlogPosts(offset = 0) {
  return await db.query.posts.findMany({
    limit: 10,
    offset: offset,
    orderBy: (post, { desc }) => desc(post.publishedAt),
    where: (post, { eq }) => eq(post.isPublished, true),
    with: {
      postTags: {
        with: {
          tag: true,
        },
      },
    },
  });
}
