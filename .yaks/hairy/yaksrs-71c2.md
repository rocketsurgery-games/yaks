---
id: yaksrs-71c2
title: Search tokenization improvements
type: feature
priority: 2
created: '2026-08-23T02:49:59Z'
updated: '2026-08-29T17:34:35Z'
labels:
- search
---

It would be really helpful if yak searches could be a bit more contextually aware. Some examples:
- When I have a field like `source: ...github/issues/123`, it would be great if `123` matched this.
  We might also consider special forms like `source:123` that use tokenization rules to limit the scope of the search.

---
▸ 2026-08-29T17:34:35Z
Also: multiple tokens vs n-grams, so you can clarify "this exact sequence of tokens" vs "this", "that", and "the other" dispersed throughout the text.
