import BlogPost from '~/app/_components/blog-post';

export default function BlogPostPage({
  params: { slug: slug },
}: {
  params: { slug: string },
}) {
  return (
    <BlogPost slug={slug} />
  );
}
