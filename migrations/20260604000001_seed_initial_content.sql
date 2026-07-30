INSERT INTO posts (title, slug, body, preview, published_at) VALUES
  (
    'I hate title case',
    'i-hate-title-case',
    'Title Case Is Bad And You Should Feel Bad.',
    'I hate title case',
    NOW()
  );

INSERT INTO tags (name, slug) VALUES
  ('letter case', 'letter-case'),
  ('title case', 'title-case');

INSERT INTO post_tags (post_id, tag_id)
SELECT posts.id, tags.id
FROM posts
CROSS JOIN tags
WHERE posts.slug = 'i-hate-title-case'
  AND tags.slug IN ('letter-case', 'title-case');
