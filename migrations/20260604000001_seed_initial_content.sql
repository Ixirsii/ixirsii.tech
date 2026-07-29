INSERT INTO posts (title, slug, body, preview, published_at) VALUES
  (
    'I hate title case',
    'i-hate-title-case',
    'Title Case Is Bad And You Should Feel Bad.\n\nSometimes the right hill to die on is a stylistic one.',
    'A short rant about title case, readability, and why sentence case wins.',
    NOW() - INTERVAL 30 DAY
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
