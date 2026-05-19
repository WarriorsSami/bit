set shell := ["bash", "-euo", "pipefail", "-c"]

demo_dir := "/tmp/bit-demo"
bit     := justfile_directory() / "target/release/bit"

export NO_PAGER := "1"
export GIT_AUTHOR_NAME := "Demo User"
export GIT_AUTHOR_EMAIL := "demo@example.com"
export GIT_AUTHOR_DATE := "2024-01-01 10:00:00 +0000"

# Build bit, then run the full demo (act1 → act2 → act3)
demo: build act1 act2 act3
    @echo "\n✅  Demo complete. Clean up with: just clean"

# Build the release binary
build:
    cargo build --release

# Act 1 — init, add, status, commit
act1:
    rm -rf {{demo_dir}} && mkdir -p {{demo_dir}}
    cd {{demo_dir}} && {{bit}} init
    cd {{demo_dir}} && echo "hello world" > app.txt && echo "fn add(a, b) { a + b }" > lib.txt
    cd {{demo_dir}} && {{bit}} status --porcelain
    cd {{demo_dir}} && {{bit}} add .
    cd {{demo_dir}} && {{bit}} status --porcelain
    cd {{demo_dir}} && {{bit}} commit -m "Initial commit"

# Act 2 — diff, log, branch, checkout
act2:
    cd {{demo_dir}} && echo "hello bit" >> app.txt
    cd {{demo_dir}} && {{bit}} diff
    cd {{demo_dir}} && {{bit}} add app.txt
    cd {{demo_dir}} && {{bit}} diff --cached
    cd {{demo_dir}} && {{bit}} commit -m "Update app"
    cd {{demo_dir}} && {{bit}} log --oneline
    cd {{demo_dir}} && {{bit}} branch create feature
    cd {{demo_dir}} && {{bit}} branch list -v
    cd {{demo_dir}} && {{bit}} checkout feature
    cd {{demo_dir}} && echo "new feature" > feature.txt && {{bit}} add . && {{bit}} commit -m "Add feature"
    cd {{demo_dir}} && {{bit}} checkout master

# Act 3 — fast-forward merge, conflict, resolution
act3:
    cd {{demo_dir}} && {{bit}} merge feature -m "Merge feature"
    cd {{demo_dir}} && echo "master's version" > app.txt && {{bit}} add . && {{bit}} commit -m "Master change"
    cd {{demo_dir}} && {{bit}} checkout feature && echo "feature's version" > app.txt && {{bit}} add . && {{bit}} commit -m "Feature change" && {{bit}} checkout master
    -cd {{demo_dir}} && {{bit}} merge feature -m "Merge diverged"
    cd {{demo_dir}} && cat app.txt
    cd {{demo_dir}} && {{bit}} status --porcelain
    cd {{demo_dir}} && echo "resolved version" > app.txt && {{bit}} add app.txt
    cd {{demo_dir}} && {{bit}} merge --continue
    cd {{demo_dir}} && {{bit}} log --oneline

# Remove the scratch directory
clean:
    rm -rf {{demo_dir}}
