import BlogPost from '~/app/_components/blog-post';
import { Modal } from './modal';

export default function BlogModal({
  params: { slug: slug },
}: {
  params: { slug: string };
}) {
  return (
    <Modal>
      <div className="overflow-y-scroll">
        <BlogPost slug={slug} />
      </div>
    </Modal>
  );
}
