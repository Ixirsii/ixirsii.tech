# CSS Audit Implementation Plan

This document records the decisions from the July 2026 CSS consistency audit.
Use it as the implementation handoff for future sessions. The goal is
consistency by semantic role, not uniform styling at the expense of readability
or visual hierarchy.

## Design Decisions

### Blockquotes

Keep blockquotes square. Their square geometry and leading border distinguish
editorial content from rounded cards, controls, alerts, and elevated panels.
If they are softened later, prefer rounding only the trailing corners rather
than making them look like cards.

Generalize multi-paragraph spacing so Markdown blockquotes outside the About
page do not run paragraphs together.

### Layout Widths

Keep the shared header and page shell at Tailwind's `max-w-5xl` (64rem). Add
the same responsive horizontal gutters to both:

```css
@apply px-4;
@apply sm:px-6;
```

Add a centered `.reading-column` with a maximum width of 45rem (720px) for
long-form content. This is close to the Cloudflare Blog's approximately 715px
paragraph width and remains substantially wider than Daring Fireball's 425px
primary text column.

```css
.reading-column {
  width: 100%;
  max-width: 45rem;
  margin-inline: auto;
}
```

Apply the reading column to:

- Blog detail articles, including title and metadata
- Full posts shown on the Home page
- Code project detail pages
- Hall of Fame narrative content
- The About quote, biography, and callout, but not the About links grid

Do not constrain:

- Header and navigation
- Blog, tag, and code indexes
- About link grids
- Admin pages and editors
- Tag cloud
- Admin Markdown preview panels

Keep `.reading-column` separate from `.prose-content`. Rendered Markdown in
cards and admin previews needs prose formatting without inheriting the reading
column width.

### Catppuccin And Contrast

The manually defined Latte, Frappe, Macchiato, and Mocha values in
`src/style/main.css` match the canonical Catppuccin palette. The contrast issue
is not an import error: Catppuccin does not guarantee that every accent passes
WCAG contrast as normal-sized foreground text on every background.

The current Tailwind v4 `@theme inline` configuration generates utilities only
for explicitly declared tokens such as `text-ctp-sapphire`. It does not
generate numbered variants such as `text-ctp-sapphire-800`. Slash modifiers
such as `text-ctp-sapphire/80` change opacity rather than lightness and do not
solve contrast.

The `@catppuccin/tailwindcss` package synthesizes accent tints and shades; these
are not additional canonical palette entries. For example, its generated Latte
Sapphire 800 still has only about 4.20:1 contrast on Latte Base, while Sapphire
900 reaches about 5.22:1.

Do not add a Node/Sass toolchain or vendor the package's complete generated
scale solely for this work. Preserve the canonical raw palette and introduce
theme-specific semantic Tailwind tokens instead, including:

- Link foreground
- Focus indicator
- Muted small text
- Semantic status foregrounds

Use generated Catppuccin shade values selectively where useful. Keep raw accent
colors for decoration, borders, backgrounds, icons, and text combinations that
pass contrast requirements.

For the tag cloud, keep small tag text readable and move color variation to a
border or background where necessary. Color assignment should also be stable
for a given tag rather than changing when filtering alters its list position.

## Accepted Changes

1. Preserve canonical Catppuccin values and add accessible semantic color
   tokens for every theme.
2. Add responsive shell gutters shared by the header and main content.
3. Add the centered 45rem reading column described above.
4. Add a scoped `.prose-content` component for rendered Markdown.
5. Restore unordered, ordered, and nested Markdown list presentation.
6. Use approximately `1.7` line height for long-form rendered prose.
7. Add a shared, high-contrast `:focus-visible` treatment.
8. Unify equivalent anchor and native-button dimensions. Choose one sensible
   default padding and use explicit variants only when the context requires it.
9. Add an accessible destructive-action button variant.
10. Finish the theme menu with a rounded elevated surface, padded rows, proper
    button semantics, keyboard support, and visible focus states.
11. Increase header icon-link hit areas and give icon-only links accessible
    names.
12. Generalize multi-paragraph blockquote spacing while keeping square corners.
13. Remove trailing margins from direct final children of bounded cards and
    panels where appropriate. Do not apply this indiscriminately to articles.
14. Make About links one column on narrow screens and defer the three-column
    wide group until sufficient space is available.
15. Delay the dense admin tag-row columns to a breakpoint with enough room.
16. Move global element defaults into `@layer base` and reusable classes into
    `@layer components` so utilities can intentionally override them.
17. Remove the empty `src/style/code.css` file and its import.
18. Replace login-prefixed classes reused by admin templates with shared,
    semantic component names. All consumers are internal, so do not add
    compatibility aliases.
19. Respect `prefers-reduced-motion` for the theme expansion indicator and
    loading spinner.
20. Give `h6` an explicit weight consistent with the heading hierarchy.

## Deferred Changes

Do not include these in the current CSS consistency work unless explicitly
requested:

- Complete the missing `.admin-form-field` structural layout and widths.
- Constrain and center the login card.
- Connect the unused loading component to HTMX or remove it.

## Recommended Implementation Order

### 1. Cascade Foundation

- Move element defaults into `@layer base`.
- Move reusable classes into `@layer components`.
- Remove `src/style/code.css` and its import.

This should happen first so later changes are implemented against the intended
cascade rather than relying on accidental unlayered precedence.

### 2. Semantic Colors

- Add theme-specific link, focus, muted-text, and status tokens.
- Update existing foreground usages to consume semantic tokens.
- Make tag-cloud text and color treatment accessible and stable.
- Verify foreground/background combinations on Base, Mantle, and Surface 0 in
  all four themes.

### 3. Layout Foundations

- Add matching `px-4 sm:px-6` gutters to the header and main shell.
- Add `.reading-column` at 45rem.
- Apply it only to the long-form regions listed above.

### 4. Rendered Prose

- Add `.prose-content` around every rendered Markdown fragment.
- Restore list markers, indentation, nesting, and item spacing.
- Set the long-form prose line height to approximately `1.7`.
- Generalize multi-paragraph blockquote spacing.
- Normalize direct final-child margins in bounded prose panels.

### 5. Keyboard Interaction

- Add a consistent, high-contrast `:focus-visible` language.
- Ensure form focus is more than a subtle border-color change.
- Add reduced-motion handling for the theme arrow and spinner.

### 6. Action Components

- Consolidate equivalent anchor and native-button styling.
- Normalize dimensions and text behavior.
- Add the destructive action variant.

### 7. Theme And Header Controls

- Replace clickable theme list items with semantic controls.
- Finish the dropdown's radius, border, spacing, hover, and focus behavior.
- Increase header icon target sizes and add accessible names.

### 8. Shared Naming

- Replace login-prefixed classes used outside authentication with shared names.
- Update all templates directly without backward-compatibility aliases.

### 9. Responsive Refinements

- Adjust the About links grid breakpoints.
- Adjust the admin tag-row breakpoint.
- Check header wrapping and navigation targets at intermediate widths.

### 10. Final Cleanup

- Add the explicit `h6` weight.
- Apply bounded-panel trailing-margin rules.
- Check hover transitions for equivalent surfaced controls.

## Verification Checklist

- Build the Tailwind output through the repository's normal build process.
- Run `cargo fmt`, `cargo check`, and `cargo clippy` when the local Tailwind CLI
  and required environment are available.
- Verify Latte, Frappe, Macchiato, and Mocha independently.
- Check links, muted metadata, placeholders, focus rings, status badges, and
  tags against every background where they appear.
- Verify keyboard navigation through the header, theme menu, forms, cards, and
  actions.
- Verify reduced-motion behavior.
- Check mobile, tablet, and desktop layouts, including intermediate widths near
  responsive breakpoints.
- Check blog detail, Home posts, About, code project detail, and Hall of Fame at
  the 45rem reading width.
- Check Markdown paragraphs, headings, unordered lists, ordered lists, nested
  lists, blockquotes, inline code, and code blocks in public pages and admin
  previews.
- Confirm full-width grids and admin interfaces still use the 64rem shell.
- Confirm template utilities can override layered base and component defaults.
