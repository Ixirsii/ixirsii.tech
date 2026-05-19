-- Add migration script here
INSERT INTO post_tags (post_id, tag_id) VALUES (
  (SELECT id FROM posts WHERE slug = 'i-hate-title-case'),
  (SELECT id FROM tags WHERE slug = 'letter-case')
);
INSERT INTO post_tags (post_id, tag_id) VALUES (
  (SELECT id FROM posts WHERE slug = 'i-hate-title-case'),
  (SELECT id FROM tags WHERE slug = 'title-case')
);
