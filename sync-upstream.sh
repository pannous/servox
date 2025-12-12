#!/bin/bash
# Sync with upstream servo/servo:main before starting work

set -e  # Exit on error

echo "🔄 Syncing with upstream servo/servo:main..."


# Fetch latest from upstream
echo "📥 Fetching from upstream..."
git remote update
git pull --all -s recursive -X theirs
git fetch upstream

# Check for uncommitted changes
git commit -a --all --allow-empty-message -m 'before sync'

# Get current branch
CURRENT_BRANCH=$(git branch --show-current)
echo "📍 Current branch: $CURRENT_BRANCH"


# Merge upstream/main into current branch
echo "🔀 Merging upstream/main into $CURRENT_BRANCH..."
if git merge upstream/main --no-edit -m "chore: merge upstream servo/servo:main"; then
    echo "✅ Successfully merged upstream changes"

    # Restore stashed changes if any
    if [ "$STASHED" = true ]; then
        echo "📦 Restoring stashed changes..."
        git stash pop
    fi

    # Show summary
    echo ""
    echo "📊 Summary:"
    git log --oneline -5

    exit 0
else
    echo "❌ Merge conflict detected!"
    echo "Please resolve conflicts manually, then run:"
    echo "  git merge --continue"

    if [ "$STASHED" = true ]; then
        echo "  git stash pop  # to restore your stashed changes"
    fi

    exit 1
fi
