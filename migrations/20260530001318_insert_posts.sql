INSERT INTO posts (title, slug, body, preview, published_at) VALUES
  (
    'I hate title case',
    'i-hate-title-case',
    'Title Case Is Bad And You Should Feel Bad.\n\nSometimes the right hill to die on is a stylistic one.',
    'A short rant about title case, readability, and why sentence case wins.',
    NOW() - INTERVAL 30 DAY
  ),
  (
    'Why I use HTMX here',
    'why-i-use-htmx-here',
    'This site does not need a giant frontend stack.\n\n- Server-rendered HTML is enough for most pages\n- HTMX adds just enough interactivity\n- The result stays simple to maintain',
    'A quick note on why HTMX is the right size for this site.',
    NOW() - INTERVAL 14 DAY
  ),
  (
    'Building this site in Rust',
    'building-this-site-in-rust',
    'The current stack is Rust, Axum, Askama, SQLx, and MySQL.\n\nIt is fast, direct, and pleasant to deploy.',
    'A stack overview for the site and the tradeoffs behind it.',
    NOW() - INTERVAL 7 DAY
  ),
  (
    'Draft posts are a feature',
    'draft-posts-are-a-feature',
    'Not every post should go live immediately.\n\nDrafts make it easier to iterate, rewrite, and leave ideas half-finished until they are ready.',
    'A draft about why unpublished posts are useful during writing.',
    NULL
  ),
  (
    'Future post rollout',
    'future-post-rollout',
    'Scheduled publishing is useful for launches, announcements, and keeping a posting cadence without manual intervention.',
    'A scheduled post used to verify future publish dates stay hidden.',
    NOW() + INTERVAL 7 DAY
  ),
  (
    'Tag page sanity check',
    'tag-page-sanity-check',
    'This post exists mostly to exercise the tag pages.\n\nIt should show up wherever published posts are listed and counted.',
    'Seed content for verifying tag listings and counts.',
    NOW() - INTERVAL 3 DAY
  );
