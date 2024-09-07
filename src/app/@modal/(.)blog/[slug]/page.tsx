import BlogPost from '~/app/_components/blog-post';
import { getBlogPost } from "~/server/queries";
import { Modal } from './modal';

export default async function BlogModal({
  params: { slug: slug },
}: {
  params: { slug: string };
}) {
  const blogPost = await getBlogPost(slug);

  if (blogPost !== undefined) {
    return (
      <Modal>
        <div className="overflow-y-scroll">
          <BlogPost blogPost={blogPost} />
        </div>
      </Modal>
    );
  } else {
    return null;
  }

}
