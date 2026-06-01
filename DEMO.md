# bit — 2-Minute Interactive Demo

A quick walkthrough of every porcelain command: `init`, `add`, `status`, `commit`, `diff`, `branch`, `checkout`, `log`, and `merge` — plus a compatibility act showing that `git` reads everything `bit` creates.

## Setup

```sh
cargo build --release
```

Run the full demo, or individual acts:

```sh
just demo          # build + act1 + act2 + act3 + act4
just act1          # just Act 1
just act2          # just Act 2 (requires act1)
just act3          # just Act 3 (requires act1 + act2)
just act4          # just Act 4 (requires act1 + act2 + act3)
just clean         # remove /tmp/bit-demo
```

Or follow the steps below manually — `cd /tmp/bit-demo` after running `just act1` to pick up from any point.

---

## Act 1 — Create a Project

> **Quick run:** `just act1`

### 1. Initialize a repository

```sh
bit init
```

```
Initialized empty Git repository in /tmp/bit-demo
```

### 2. Create some files

```sh
echo "hello world" > app.txt
echo "fn add(a, b) { a + b }" > lib.txt
```

### 3. Check status — untracked files

```sh
bit status --porcelain
```

```
?? app.txt
?? lib.txt
```

### 4. Stage everything

```sh
bit add .
```

### 5. Check status — staged files

```sh
bit status --porcelain
```

```
A  app.txt
A  lib.txt
```

### 6. First commit

```sh
bit commit -m "Initial commit"
```

```
[(root-commit) <sha>] Initial commit
```

> **Rollback:** `rm -rf .git` to start over.

---

## Act 2 — Evolve and Inspect

> **Quick run:** `just act2` (requires act1)

### 7. Modify a file and view the workspace diff

```sh
echo "hello bit" >> app.txt
bit diff
```

```
diff --git a/app.txt b/app.txt
index <oid>..<oid> 100644
--- a/app.txt
+++ b/app.txt
@@ -1,1 +1,2 @@
 hello world
+hello bit
```

### 8. Stage the change and view the cached diff

```sh
bit add app.txt
bit diff --cached
```

Same hunk as above — now it shows the staged change against HEAD.

### 9. Second commit

```sh
bit commit -m "Update app"
```

```
[<sha>] Update app
```

### 10. View commit history

```sh
bit log --oneline
```

```
<sha> (HEAD -> master) Update app

<sha> Initial commit
```

### 11. Create a branch

```sh
bit branch create feature
```

### 12. List branches (verbose)

```sh
bit branch list -v
```

```
  feature  <sha> Update app
* master   <sha> Update app
```

### 13. Switch to the feature branch

```sh
bit checkout feature
```

```
Switched to branch 'feature'
```

### 14. Add a commit on the feature branch

```sh
echo "new feature" > feature.txt
bit add .
bit commit -m "Add feature"
```

```
[<sha>] Add feature
```

### 15. Switch back to master

```sh
bit checkout master
```

```
Switched to branch 'master'
```

`feature.txt` is gone — it only exists on the `feature` branch.

> **Rollback:** `bit checkout master` returns to a known state.

---

## Act 3 — Merge

> **Quick run:** `just act3` (requires act1 + act2)

### 16. Fast-forward merge

Since `master` hasn't diverged, merging `feature` is a fast-forward:

```sh
bit merge feature -m "Merge feature"
```

```
Fast-forwarding <sha> to <sha>
```

`feature.txt` is now on master. No merge commit is created.

### 17. Set up a conflict — master side

```sh
echo "master's version" > app.txt
bit add .
bit commit -m "Master change"
```

### 18. Set up a conflict — feature side

```sh
bit checkout feature
echo "feature's version" > app.txt
bit add .
bit commit -m "Feature change"
bit checkout master
```

### 19. Attempt the merge — conflict!

```sh
bit merge feature -m "Merge diverged"
```

```
Auto-merging app.txt failed
CONFLICT (content): Merge conflict in app.txt
Error: Merge conflict in: app.txt — fix conflicts then commit
```

### 20. Inspect the conflict markers

```sh
cat app.txt
```

```
<<<<<<< HEAD
master's version
=======
feature's version
>>>>>>> feature
```

### 21. Check status during a conflict

```sh
bit status --porcelain
```

```
UU app.txt
```

`UU` = both sides modified the same file.

### 22. Resolve the conflict

```sh
echo "resolved version" > app.txt
bit add app.txt
```

### 23. Complete the merge

```sh
bit merge --continue
```

```
[<sha>] Merge diverged
```

The merge commit uses the message saved from step 19.

### 24. View the final history

```sh
bit log --oneline
```

```
<sha> (HEAD -> master) Merge diverged

<sha> (feature) Feature change

<sha> Master change

<sha> Add feature

<sha> Update app

<sha> Initial commit
```

> **Rollback:** Delete `.git/MERGE_HEAD` and `.git/MERGE_MSG` to abort a conflicted merge.

---

## Act 4 — bit-git Compatibility

> **Quick run:** `just act4` (requires act1 + act2 + act3)

Everything `bit` writes is native Git — objects, refs, index, and merge commits are all standard format. Your existing `git` tools work without any conversion.

### 25. git reads bit's full history

```sh
git log --oneline --graph --all
```

```
*   <sha> Merge diverged
|\
| * <sha> Feature change
* | <sha> Master change
|/
* <sha> Add feature
* <sha> Update app
* <sha> Initial commit
```

Git sees the complete DAG, including the merge topology from Act 3.

### 26. git inspects bit's objects

```sh
git cat-file -p HEAD
```

```
tree <sha>
parent <sha>
parent <sha>
author Demo User <demo@example.com> 1704103200 +0000
committer Demo User <demo@example.com> 1704103200 +0000

Merge diverged
```

Both parents of the merge commit are intact — first parent is the master lineage, second is the feature branch.

### 27. git sees bit-created branches

```sh
git branch -v
```

```
  feature <sha> Feature change
* master  <sha> Merge diverged
```

### 28. git diff works on a bit repo

```sh
echo "git-side change" >> app.txt
git diff
```

```
diff --git a/app.txt b/app.txt
index <oid>..<oid> 100644
--- a/app.txt
+++ b/app.txt
@@ -1 +1,2 @@
 resolved version
+git-side change
```

### 29. git commits on top of bit's history

```sh
git add app.txt
git commit -m "Git-side edit"
```

```
[master <sha>] Git-side edit
```

The git commit chains onto bit's merge commit seamlessly.

### 30. Mixed bit + git history

```sh
git log --oneline --graph --all
```

```
* <sha> Git-side edit
*   <sha> Merge diverged
|\
| * <sha> Feature change
* | <sha> Master change
|/
* <sha> Add feature
* <sha> Update app
* <sha> Initial commit
```

bit-created and git-created commits are indistinguishable in the log.

### 31. git clone produces a faithful copy

```sh
git clone /tmp/bit-demo /tmp/bit-demo-clone
cd /tmp/bit-demo-clone
git log --oneline --graph --all
```

```
* <sha> Git-side edit
*   <sha> Merge diverged
|\
| * <sha> Feature change
* | <sha> Master change
|/
* <sha> Add feature
* <sha> Update app
* <sha> Initial commit
```

The full history, branches, and objects survive a clone — bit repos are real Git repos.

---

## Cleanup

```sh
just clean
```

## Command Reference

| Command | Example | Description |
|---------|---------|-------------|
| `bit init` | `bit init` | Initialize a new repository |
| `bit add` | `bit add .` | Stage files |
| `bit status` | `bit status --porcelain` | Show working tree status |
| `bit commit` | `bit commit -m "msg"` | Create a commit |
| `bit diff` | `bit diff --cached` | Show changes |
| `bit log` | `bit log --oneline` | View commit history |
| `bit branch` | `bit branch create name` | Manage branches |
| `bit checkout` | `bit checkout branch` | Switch branches |
| `bit merge` | `bit merge branch -m "msg"` | Merge branches |
