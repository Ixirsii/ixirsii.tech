// Example model schema from the Drizzle docs
// https://orm.drizzle.team/docs/sql-schema-declaration

import { defineConfig } from "drizzle-kit";
import { relations, sql } from "drizzle-orm";
import { index, pgTableCreator, primaryKey } from "drizzle-orm/pg-core";

import { env } from "~/env";

const createIndex = (name: string) => `ixirsii.tech_${name}_idx`;

/**
 * This is an example of how to use the multi-project schema feature of Drizzle ORM.
 * Use the same database instance for multiple projects.
 *
 * @see https://orm.drizzle.team/docs/goodies#multi-project-schema
 */
const createTable = pgTableCreator((name) => `ixirsii.tech_${name}`);

export const posts = createTable(
  "post",
  (d) => ({
    id: d.integer().primaryKey().generatedByDefaultAsIdentity(),
    createdAt: d
      .timestamp({ withTimezone: true })
      .default(sql`CURRENT_TIMESTAMP`)
      .notNull(),
    updatedAt: d.timestamp({ withTimezone: true }).$onUpdate(() => new Date()),
    body: d.text().notNull(),
    isPublished: d.boolean().default(false).notNull(),
    preview: d.varchar({ length: 512 }).notNull(),
    publishedAt: d.timestamp({ withTimezone: true }),
    slug: d.varchar({ length: 255 }).notNull(),
    title: d.varchar({ length: 255 }).notNull(),
  }),
  (table) => [
    index(createIndex("slug")).on(table.slug),
    index(createIndex("title")).on(table.title),
  ],
);

export const tags = createTable("tag", (d) => ({
  id: d.integer().primaryKey().generatedByDefaultAsIdentity(),
  name: d.varchar({ length: 255 }).notNull(),
}));

export const postTags = createTable(
  "post_tag",
  (d) => ({
    postId: d
      .integer()
      .notNull()
      .references(() => posts.id),
    tagId: d
      .integer()
      .notNull()
      .references(() => tags.id),
  }),
  (table) => [primaryKey({ columns: [table.postId, table.tagId] })],
);

/* ******************************** Relations ******************************* */

export const postsRelations = relations(posts, ({ many }) => ({
  postTags: many(postTags),
}));

export const tagsRelations = relations(tags, ({ many }) => ({
  postTags: many(postTags),
}));

export const postTagsRelations = relations(postTags, ({ one }) => ({
  post: one(posts, {
    fields: [postTags.postId],
    references: [posts.id],
  }),
  tag: one(tags, {
    fields: [postTags.tagId],
    references: [tags.id],
  }),
}));
