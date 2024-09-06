// Example model schema from the Drizzle docs
// https://orm.drizzle.team/docs/sql-schema-declaration

import { relations, sql } from "drizzle-orm";
import {
  boolean,
  index,
  integer,
  pgTableCreator,
  serial,
  text,
  timestamp,
  varchar,
} from "drizzle-orm/pg-core";

/**
 * This is an example of how to use the multi-project schema feature of Drizzle
 * ORM. Use the same database instance for multiple projects.
 *
 * @see https://orm.drizzle.team/docs/goodies#multi-project-schema
 */
export const createTable = pgTableCreator((name) => `ixirsii.tech_${name}`);

/* ********************************* Tables ********************************* */

export const tags = createTable("tag", {
  id: serial("id").primaryKey(),
  name: varchar("name", { length: 256 }).notNull(),
});

export const users = createTable("user", {
  id: serial("id").primaryKey(),
  about: text("about"),
  email: varchar("email", { length: 256 }).notNull(),
  name: varchar("name", { length: 256 }).notNull(),
});

export const links = createTable("link", {
  id: serial("id").primaryKey(),
  name: varchar("name", { length: 256 }).notNull(),
  url: text("url").notNull(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
});

export const posts = createTable(
  "post",
  {
    id: serial("id").primaryKey(),
    authorId: integer("author_id")
      .notNull()
      .references(() => users.id),
    body: text("body").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .default(sql`CURRENT_TIMESTAMP`)
      .notNull(),
    isPublished: boolean("is_published").default(false).notNull(),
    preview: varchar("preview", { length: 256 }),
    slug: varchar("slug", { length: 256 }).notNull(),
    title: text("title").notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .default(sql`CURRENT_TIMESTAMP`)
      .notNull(),
  },
  (table) => ({
    slugIndex: index("slug_idx").on(table.slug),
  }),
);

export const postTags = createTable("post_tag", {
  id: serial("id").primaryKey(),
  postId: integer("post_id")
    .notNull()
    .references(() => posts.id),
  tagId: integer("tag_id")
    .notNull()
    .references(() => tags.id),
});

/* ******************************** Relations ******************************* */

export const usersRelations = relations(users, ({ many }) => ({
  links: many(links),
  posts: many(posts),
}));

export const linksRelations = relations(links, ({ one }) => ({
  user: one(users, {
    fields: [links.userId],
    references: [users.id],
  }),
}));

export const postsRelations = relations(posts, ({ many, one }) => ({
  author: one(users, {
    fields: [posts.authorId],
    references: [users.id],
  }),
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
