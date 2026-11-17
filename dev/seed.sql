-- Development fixtures only. Run migrations first, then:
--     docker compose run --rm seed
-- The Compose service always connects to its local postgres service and never
-- uses DATABASE_URL. This file is not embedded in the application or migrations.
-- Rerunning adds missing fixtures without resetting data or overwriting edits.
BEGIN;

DO $$
BEGIN
    IF current_database() <> 'ixirsii_dev' OR current_user <> 'ixirsii' THEN
        RAISE EXCEPTION 'Development fixtures require the ixirsii_dev database and ixirsii role';
    END IF;
END
$$;

INSERT INTO tags (name, slug)
VALUES
    ('Development', 'dev-development'),
    ('Rust', 'dev-rust'),
    ('PostgreSQL', 'dev-postgresql'),
    ('Unused fixture tag', 'dev-unused'),
    ('Unpublished fixture tag', 'dev-unpublished')
ON CONFLICT DO NOTHING;

-- Twelve published posts span two pages. Consecutive pairs share a publication
-- time to exercise deterministic ID tie-breaking. Posts 11 and 12 have no tags.
INSERT INTO posts (title, slug, body, preview, published_at)
SELECT
    'Development post ' || lpad(n::TEXT, 2, '0'),
    'dev-post-' || lpad(n::TEXT, 2, '0'),
    format(E'## Development post %s\n\nThis fixture exercises **Markdown**, [links](https://www.rust-lang.org/), and pagination.\n\n- First list item\n- Second list item\n\n```rust\nlet post_number = %s;\n```', n, n),
    CASE WHEN n % 3 = 0 THEN NULL ELSE 'A local development post for checking pagination and tags.' END,
    CURRENT_TIMESTAMP - ((14 - n) / 2) * INTERVAL '1 day'
FROM generate_series(1, 12) AS fixture(n)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO posts (title, slug, body, preview, published_at)
VALUES
    ('Development draft', 'dev-draft', 'This draft should only appear in the admin interface.', NULL, NULL),
    ('Development scheduled post', 'dev-scheduled', 'This post must stay hidden until its scheduled publication date.', 'A future publication fixture.', TIMESTAMPTZ '2099-01-01 12:00:00+00')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO post_tags (post_id, tag_id)
SELECT posts.id, tags.id
FROM (
    VALUES
        ('dev-post-01', 'dev-development'),
        ('dev-post-01', 'dev-rust'),
        ('dev-post-01', 'dev-postgresql'),
        ('dev-post-02', 'dev-development'),
        ('dev-post-03', 'dev-development'),
        ('dev-post-03', 'dev-rust'),
        ('dev-post-04', 'dev-development'),
        ('dev-post-05', 'dev-development'),
        ('dev-post-05', 'dev-postgresql'),
        ('dev-post-06', 'dev-development'),
        ('dev-post-07', 'dev-development'),
        ('dev-post-07', 'dev-rust'),
        ('dev-post-08', 'dev-development'),
        ('dev-post-09', 'dev-development'),
        ('dev-post-09', 'dev-postgresql'),
        ('dev-post-10', 'dev-development'),
        ('dev-draft', 'dev-unpublished'),
        ('dev-scheduled', 'dev-unpublished')
) AS fixture(post_slug, tag_slug)
JOIN posts ON posts.slug = fixture.post_slug
JOIN tags ON tags.slug = fixture.tag_slug
ON CONFLICT DO NOTHING;

-- Local login: developer / local-development-only
-- Fixed Argon2id hash is intentionally public and restricted to these fixtures.
INSERT INTO admin_users (username, password_hash)
VALUES ('developer', '$argon2id$v=19$m=19456,t=2,p=1$aXhpcnNpaS1kZXYtb25seQ$8k8VcWHiaaBqwzl4afJ3L7QMp6nhuG4FxUX4O5Qu964')
ON CONFLICT (username) DO NOTHING;

COMMIT;
