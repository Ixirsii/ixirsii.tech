import Post from "~/app/blog/_components/post";
import { listBlogPosts } from "~/server/db/queries";

export const dynamic = "force-dynamic";

export default async function Blog() {
  const posts = await listBlogPosts();

  return (
    <>
      <div className="grid grid-cols-2 gap-4">
        {posts.map((post) => (
          <Post
            key={post.id}
            preview={post.preview ?? post.body}
            publishedAt={post.publishedAt}
            slug={post.slug}
            tags={post.postTags.map(postTag => postTag.tag)}
            title={post.title}
          />
        ))}
      </div>
    </>
  );
}

