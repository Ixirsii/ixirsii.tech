"use server";

import { db } from "~/server/db";

export async function getBlogPost(slug: string) {
  return db.query.posts.findFirst({
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

/**
 * Post with tags.
 */
export type BlogPostWithTags = Awaited<ReturnType<typeof getBlogPost>>;

export async function listBlogPosts(offset = 0, limit = 10) {
  return db.query.posts.findMany({
    limit: limit,
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

/**
 * List of posts with tags.
 */
export type BlogPostsWithTags = Awaited<ReturnType<typeof listBlogPosts>>;
