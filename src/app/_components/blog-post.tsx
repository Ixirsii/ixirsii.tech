import Markdown from "react-markdown";
import { getBlogPost } from "~/server/queries";

export default async function BlogPost({ slug }: { slug: string }) {
  const blogPost = await getBlogPost(slug);

  return (<Markdown>{blogPost?.body}</Markdown>);
}
