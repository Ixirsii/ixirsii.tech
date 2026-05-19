-- Add migration script here
INSERT INTO posts (title, slug, body, preview, published_at) VALUES ('I hate title case', 'i-hate-title-case', 'Title Case Is Bad And You Should Feel Bad', NULL, NOW());
