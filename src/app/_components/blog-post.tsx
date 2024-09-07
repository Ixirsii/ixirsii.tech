import Markdown from "react-markdown";
import type * as schema from "~/server/db/schema";

export default async function BlogPost({
  blogPost,
  tags
}: {
  blogPost: typeof schema.posts.$inferSelect;
  tags?: string;
}) {
  if (blogPost != null) {
    return (
      <>
        <Markdown>{blogPost.body}</Markdown>
        <div>
          {tags !== undefined && (<> Tags: {tags} </>) }
        </div>
      </>
    );
  } else {
    return null;
  }
}
