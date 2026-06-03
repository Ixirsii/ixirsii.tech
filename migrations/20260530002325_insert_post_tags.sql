INSERT INTO post_tags (post_id, tag_id) VALUES
  (
    (SELECT id FROM posts WHERE slug = 'i-hate-title-case'),
    (SELECT id FROM tags WHERE slug = 'letter-case')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'i-hate-title-case'),
    (SELECT id FROM tags WHERE slug = 'title-case')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'why-i-use-htmx-here'),
    (SELECT id FROM tags WHERE slug = 'htmx')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'why-i-use-htmx-here'),
    (SELECT id FROM tags WHERE slug = 'axum')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'why-i-use-htmx-here'),
    (SELECT id FROM tags WHERE slug = 'personal-site')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'building-this-site-in-rust'),
    (SELECT id FROM tags WHERE slug = 'rust')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'building-this-site-in-rust'),
    (SELECT id FROM tags WHERE slug = 'axum')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'building-this-site-in-rust'),
    (SELECT id FROM tags WHERE slug = 'mysql')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'building-this-site-in-rust'),
    (SELECT id FROM tags WHERE slug = 'personal-site')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'draft-posts-are-a-feature'),
    (SELECT id FROM tags WHERE slug = 'drafts')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'draft-posts-are-a-feature'),
    (SELECT id FROM tags WHERE slug = 'personal-site')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'future-post-rollout'),
    (SELECT id FROM tags WHERE slug = 'rust')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'future-post-rollout'),
    (SELECT id FROM tags WHERE slug = 'personal-site')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'tag-page-sanity-check'),
    (SELECT id FROM tags WHERE slug = 'htmx')
  ),
  (
    (SELECT id FROM posts WHERE slug = 'tag-page-sanity-check'),
    (SELECT id FROM tags WHERE slug = 'mysql')
  );
