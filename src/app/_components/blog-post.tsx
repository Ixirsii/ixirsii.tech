import Markdown from "react-markdown";
import { type getBlogPost } from "~/server/db/queries";

export default async function BlogPost({
  blogPost,
}: {
  blogPost: Awaited<ReturnType<typeof getBlogPost>> | null;
}) {
  if (blogPost != null) {
    return (
      <>
        <Markdown>{blogPost.body}</Markdown>
        <div className="text-sm">
          {blogPost.publishedAt !== null && (
            <div>{`Published at: ${blogPost.publishedAt?.toLocaleString()}`}</div>
          )}
          {blogPost.updatedAt !== null && (
            <div>{`Updated at: ${blogPost.updatedAt?.toLocaleString()}`}</div>
          )}
        </div>
      </>
    );
  } else {
    return null;
  }
}
