---
id: yaksrs-953f
title: Global search context
type: bug
priority: 3
created: '2026-08-23T14:38:59Z'
updated: '2026-08-23T14:42:08Z'
---

When searching within a list, we get a "filter: ..." header, and it sticks within the current list. But when you change lists, the header persists, but the search context doesn't.

Let's try and unify search handling in such a way that the filter: header and actual search behavior are always in sync. And then ensure that it also propagates to detail views. Once that structure's sound, we can also have the "next" affordance remember the last search and pull it back in, like in vim.
