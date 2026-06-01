set shell := ["bash", "-euo", "pipefail", "-c"]
set quiet

demo_dir := "/tmp/bit-demo"
bit      := justfile_directory() / "target/release/bit"

export NO_PAGER := "1"
export GIT_AUTHOR_NAME := "Demo User"
export GIT_AUTHOR_EMAIL := "demo@example.com"
export GIT_AUTHOR_DATE := "2024-01-01 10:00:00 +0000"

# Build bit, then run the full demo (act1 → act2 → act3 → act4)
demo: build act1 act2 act3 act4
    @echo ""
    @echo "✅  Demo complete. Clean up with: just clean"

# Build the release binary
build:
    cargo build --release

# Act 1 — init, add, status, commit
act1:
    @echo ""
    @echo "── Act 1: init, add, status, commit ──────────────"
    rm -rf {{demo_dir}} && mkdir -p {{demo_dir}}
    @echo ""
    @echo "$ bit init"
    cd {{demo_dir}} && {{bit}} init
    cd {{demo_dir}} && echo "hello world" > app.txt && echo "fn add(a, b) { a + b }" > lib.txt
    @echo "$ bit status --porcelain"
    cd {{demo_dir}} && {{bit}} status --porcelain
    cd {{demo_dir}} && {{bit}} add .
    @echo ""
    @echo "$ bit status --porcelain   (after add)"
    cd {{demo_dir}} && {{bit}} status --porcelain
    @echo ""
    @echo "$ bit commit -m 'Initial commit'"
    cd {{demo_dir}} && {{bit}} commit -m "Initial commit"

# Act 2 — diff, log, branch, checkout
act2:
    @echo ""
    @echo "── Act 2: diff, log, branch, checkout ────────────"
    cd {{demo_dir}} && echo "hello bit" >> app.txt
    @echo ""
    @echo "$ bit diff"
    cd {{demo_dir}} && {{bit}} diff
    cd {{demo_dir}} && {{bit}} add app.txt
    @echo "$ bit diff --cached"
    cd {{demo_dir}} && {{bit}} diff --cached
    @echo "$ bit commit -m 'Update app'"
    cd {{demo_dir}} && {{bit}} commit -m "Update app"
    @echo ""
    @echo "$ bit log --oneline"
    cd {{demo_dir}} && {{bit}} log --oneline
    cd {{demo_dir}} && {{bit}} branch create feature
    @echo "$ bit branch list -v"
    cd {{demo_dir}} && {{bit}} branch list -v
    @echo ""
    @echo "$ bit checkout feature"
    cd {{demo_dir}} && {{bit}} checkout feature
    cd {{demo_dir}} && echo "new feature" > feature.txt && {{bit}} add . && {{bit}} commit -m "Add feature"
    @echo ""
    @echo "$ bit checkout master"
    cd {{demo_dir}} && {{bit}} checkout master

# Act 3 — fast-forward merge, conflict, resolution
act3:
    @echo ""
    @echo "── Act 3: merge (fast-forward + conflict) ────────"
    @echo ""
    @echo "$ bit merge feature -m 'Merge feature'"
    cd {{demo_dir}} && {{bit}} merge feature -m "Merge feature"
    cd {{demo_dir}} && echo "master's version" > app.txt && {{bit}} add . && {{bit}} commit -m "Master change"
    cd {{demo_dir}} && {{bit}} checkout feature && echo "feature's version" > app.txt && {{bit}} add . && {{bit}} commit -m "Feature change" && {{bit}} checkout master
    @echo ""
    @echo "$ bit merge feature -m 'Merge diverged'   (conflict!)"
    -cd {{demo_dir}} && {{bit}} merge feature -m "Merge diverged"
    @echo ""
    @echo "$ cat app.txt"
    cd {{demo_dir}} && cat app.txt
    @echo ""
    @echo "$ bit status --porcelain"
    cd {{demo_dir}} && {{bit}} status --porcelain
    cd {{demo_dir}} && echo "resolved version" > app.txt && {{bit}} add app.txt
    @echo ""
    @echo "$ bit merge --continue"
    cd {{demo_dir}} && {{bit}} merge --continue
    @echo ""
    @echo "$ bit log --oneline"
    cd {{demo_dir}} && {{bit}} log --oneline

# Act 4 — bit-git compatibility (git reads everything bit created)
act4:
    @echo ""
    @echo "── Act 4: bit-git compatibility ──────────────────"
    @echo ""
    @echo "$ git log --oneline --graph --all"
    cd {{demo_dir}} && git log --oneline --graph --all
    @echo ""
    @echo "$ git cat-file -p HEAD"
    cd {{demo_dir}} && git cat-file -p HEAD
    @echo ""
    @echo "$ git branch -v"
    cd {{demo_dir}} && git branch -v
    @echo ""
    @echo "$ git diff   (after echo 'git-side change' >> app.txt)"
    cd {{demo_dir}} && echo "git-side change" >> app.txt && git diff
    @echo ""
    @echo "$ git add app.txt && git commit -m 'Git-side edit'"
    cd {{demo_dir}} && git add app.txt && git commit -m "Git-side edit"
    @echo ""
    @echo "$ git log --oneline --graph --all   (mixed bit + git history)"
    cd {{demo_dir}} && git log --oneline --graph --all
    @echo ""
    @echo "$ git clone /tmp/bit-demo /tmp/bit-demo-clone"
    rm -rf {{demo_dir}}-clone
    git clone {{demo_dir}} {{demo_dir}}-clone 2>&1
    cd {{demo_dir}}-clone && git log --oneline --graph --all
    rm -rf {{demo_dir}}-clone

# Remove the scratch directory
clean:
    rm -rf {{demo_dir}}
