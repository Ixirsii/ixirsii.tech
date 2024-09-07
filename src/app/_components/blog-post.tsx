import Markdown from "react-markdown";
import type * as schema from "~/server/db/schema";

export default async function BlogPost({
  blogPost,
  publishedAt,
  tags,
  updatedAt,
}: {
  blogPost: typeof schema.posts.$inferSelect;
  publishedAt?: Date;
  tags?: string;
  updatedAt?: Date;
}) {
  if (blogPost != null) {
    return (
      <>
        <Markdown>{blogPost.body}</Markdown>
        <div className="text-sm">
          {publishedAt !== undefined && (
              <div>{`Published at: ${publishedAt.toLocaleString()}`}</div>
          )}
          {updatedAt !== undefined && (
              <div>{`Updated at: ${updatedAt.toLocaleString()}`}</div>
          )}
          <div>{tags !== undefined && (<> Tags: {tags} </>) }</div>
          
        </div>
      </>
    );
  } else {
    return null;
  }
}
