import Link from "next/link";
import { listBlogPosts } from "~/server/queries";

function Post({
  createdAt,
  slug,
  title,
  updatedAt,
}: {
  createdAt: Date,
  slug: string,
  title: string,
  updatedAt: Date,
}) {
  return (
    <Link
      className="flex flex-col rounded-xl bg-teal-800/10 p-4 hover:bg-teal-800/20"
      href={slug}
    >
      <h3>{title}</h3>
      <div>{`Created at: ${createdAt.toLocaleString()}`}</div>
      <div>{`Updated at: ${updatedAt.toLocaleString()}`}</div>
    </Link>
  );
}

export default async function Blog() {
  const posts = await listBlogPosts();

  return (
    <div className="grid grid-cols-2 gap-4">
      {posts.map((post) => (
        <Post
          key={post.id}
          createdAt={post.createdAt}
          slug={post.slug}
          title={post.title}
          updatedAt={post.updatedAt}
        />
      ))}
    </div>
  );
}

