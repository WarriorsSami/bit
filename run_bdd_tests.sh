#!/bin/bash

# BDD Test Runner for bit project
# Usage: ./run_bdd_tests.sh [feature] [scenario]
# Examples:
#   ./run_bdd_tests.sh commit
#   ./run_bdd_tests.sh index
#   ./run_bdd_tests.sh all

set -e

FEATURE=${1:-all}
SCENARIO=${2:-}

echo "🧪 Running BDD tests for bit project..."

case $FEATURE in
    "commit")
        echo "📝 Running commit feature tests..."
        CUCUMBER_FEATURE=commit cargo test --test bdd_tests
        ;;
    "index")
        echo "📋 Running index feature tests..."
        CUCUMBER_FEATURE=index cargo test --test bdd_tests
        ;;
    "all")
        echo "🎯 Running all BDD feature tests..."
        echo "📝 Testing commit features..."
        CUCUMBER_FEATURE=commit cargo test --test bdd_tests
        echo ""
        echo "📋 Testing index features..."
        CUCUMBER_FEATURE=index cargo test --test bdd_tests
        ;;
    *)
        echo "❌ Unknown feature: $FEATURE"
        echo "Usage: $0 [commit|index|all]"
        exit 1
        ;;
esac

echo "✅ BDD tests completed!"
