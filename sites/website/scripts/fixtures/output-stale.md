---
title: Stale output
---

An output that no longer matches.

```nx
type User = { id:string name:string }
<User id="1" name="Ada" />
```

```nx output
<User id="1" name="Grace" />
```
