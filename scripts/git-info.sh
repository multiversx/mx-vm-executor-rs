#!/bin/bash
# Prints the git provenance of this checkout (repo, branch, commit, dirty flag).
# Usage: scripts/git-info.sh > some-file.txt
set -e

REPO_URL="https://github.com/multiversx/mx-vm-executor-rs"

cd "$(dirname "$0")/.."

echo "repo: $REPO_URL"
echo "branch: $(git rev-parse --abbrev-ref HEAD)"
echo "commit: $(git rev-parse HEAD)"
if [ -n "$(git status --porcelain)" ]; then
  echo "dirty: yes"
else
  echo "dirty: no"
fi

# Wasmer fork revision(s) pinned in vm-executor-wasmer/Cargo.toml.
# Normally a single rev shared by wasmer, wasmer-vm and wasmer-types.
WASMER_REVS=$(grep -A1 'git = "https://github.com/multiversx/wasmer"' vm-executor-wasmer/Cargo.toml \
  | sed -n 's/^rev = "\(.*\)"/\1/p' | sort -u)
if [ -z "$WASMER_REVS" ]; then
  echo "wasmer-fork-commit: none (no pinned git rev found in vm-executor-wasmer/Cargo.toml)"
else
  echo "$WASMER_REVS" | while read -r rev; do
    echo "wasmer-fork-commit: $rev"
  done
fi
