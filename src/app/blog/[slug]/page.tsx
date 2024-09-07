import BlogPost from '~/app/_components/blog-post';
import { getBlogPost } from "~/server/queries";

export default async function BlogPostPage({
  params: { slug: slug },
}: {
  params: { slug: string },
}) {
  const blogPost = await getBlogPost(slug);
  const tags = blogPost?.postTags.map((postTag) => postTag.tag)
    .map((tag) => tag.name)
    .join(", ");

  if (blogPost !== undefined) {
    return (
      <BlogPost blogPost={blogPost} tags={tags} />
    );
  } else {
    return null;
  }
}
