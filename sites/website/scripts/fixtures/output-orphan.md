---
title: Orphaned output
---

```nx output
42
```

```nx fragment
<User id="1" />
```

```nx output
<User id="1" />
```

```nx invalid
let broken(): int = { "oops" }
```

```nx output
{}
```
