#!/usr/bin/env bash
#
# validate-migrations.sh
# Validates database migration file naming conventions
#
# Usage: bash .github/scripts/validate-migrations.sh
#
# Exit codes:
#   0 - All validations passed
#   1 - Validation failures found

set -euo pipefail

MIGRATIONS_DIR="database/migrations"
EXIT_CODE=0

echo "🔍 Validating database migrations in ${MIGRATIONS_DIR}/"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# 1. Check for duplicate prefixes
# ─────────────────────────────────────────────────────────────────────────────

echo "📋 Checking for duplicate migration prefixes..."

cd "${MIGRATIONS_DIR}" || { echo "❌ Error: ${MIGRATIONS_DIR} directory not found"; exit 1; }

# Extract all prefixes (everything before first underscore)
DUPLICATES=$(ls -1 *.sql 2>/dev/null | sed 's/_.*$//' | sort | uniq -d)

if [ -n "${DUPLICATES}" ]; then
    echo "❌ FAIL: Duplicate migration prefixes found:"
    echo ""
    for prefix in ${DUPLICATES}; do
        echo "  Prefix ${prefix}:"
        ls -1 | grep "^${prefix}_" | sed 's/^/    - /'
    done
    echo ""
    echo "  Fix: Rename duplicate files to use unique sequential numbers."
    echo "  Example: If 038 is duplicate, rename one to next available (e.g., 049)."
    EXIT_CODE=1
else
    echo "✅ PASS: All migration prefixes are unique"
fi

echo ""

# ─────────────────────────────────────────────────────────────────────────────
# 2. Validate prefix format (3-digit zero-padded or timestamp)
# ─────────────────────────────────────────────────────────────────────────────

echo "🔢 Validating prefix format..."

INVALID_FORMAT=0

for file in *.sql; do
    prefix=$(echo "$file" | sed 's/_.*$//')

    # Check if it's a 3-digit number or 14-digit timestamp
    if ! [[ "$prefix" =~ ^[0-9]{3}$ ]] && ! [[ "$prefix" =~ ^[0-9]{14}$ ]]; then
        if [ $INVALID_FORMAT -eq 0 ]; then
            echo "❌ FAIL: Invalid prefix format found:"
            echo ""
        fi
        echo "  - ${file} (prefix: ${prefix})"
        echo "    Expected: 3-digit zero-padded (001-999) or 14-digit timestamp (YYYYMMDDHHMMSS)"
        INVALID_FORMAT=1
    fi
done

if [ $INVALID_FORMAT -eq 1 ]; then
    echo ""
    EXIT_CODE=1
else
    echo "✅ PASS: All prefixes are properly formatted"
fi

echo ""

# ─────────────────────────────────────────────────────────────────────────────
# 3. Check for sequential gaps (warning only)
# ─────────────────────────────────────────────────────────────────────────────

echo "🔗 Checking for gaps in sequential numbering (warnings only)..."

# Get only 3-digit prefixes, sorted numerically
SEQUENTIAL_PREFIXES=$(ls -1 *.sql 2>/dev/null | sed 's/_.*$//' | grep -E '^[0-9]{3}$' | sort -n | uniq)

if [ -n "${SEQUENTIAL_PREFIXES}" ]; then
    PREV_NUM=""
    GAPS_FOUND=0

    for num in ${SEQUENTIAL_PREFIXES}; do
        if [ -n "${PREV_NUM}" ]; then
            # Remove leading zeros for arithmetic
            PREV_INT=$((10#${PREV_NUM}))
            CURR_INT=$((10#${num}))

            if [ $((CURR_INT - PREV_INT)) -gt 1 ]; then
                if [ $GAPS_FOUND -eq 0 ]; then
                    echo "⚠️  WARNING: Gaps detected in sequential numbering:"
                    echo ""
                fi
                echo "  - Gap: ${PREV_NUM} → ${num} (missing: $(seq $((PREV_INT + 1)) $((CURR_INT - 1)) | tr '\n' ',' | sed 's/,$//'))"
                GAPS_FOUND=1
            fi
        fi
        PREV_NUM="${num}"
    done

    if [ $GAPS_FOUND -eq 0 ]; then
        echo "✅ PASS: No gaps in sequential numbering"
    else
        echo ""
        echo "  Note: Gaps are allowed but may indicate missing migrations or renumbering."
    fi
else
    echo "⚠️  No sequential migrations found (only timestamps)"
fi

echo ""

# ─────────────────────────────────────────────────────────────────────────────
# 4. Validate filename convention
# ─────────────────────────────────────────────────────────────────────────────

echo "📝 Validating filename conventions..."

INVALID_NAME=0

for file in *.sql; do
    # Check format: NNN_description.sql or YYYYMMDDHHMMSS_description.sql
    # Description must be lowercase, alphanumeric + underscores
    if ! [[ "$file" =~ ^[0-9]{3}_[a-z0-9_]+\.sql$ ]] && ! [[ "$file" =~ ^[0-9]{14}_[a-z0-9_]+\.sql$ ]]; then
        if [ $INVALID_NAME -eq 0 ]; then
            echo "❌ FAIL: Invalid filename convention:"
            echo ""
        fi
        echo "  - ${file}"
        echo "    Expected: NNN_lowercase_description.sql or YYYYMMDDHHMMSS_lowercase_description.sql"
        INVALID_NAME=1
    fi
done

if [ $INVALID_NAME -eq 1 ]; then
    echo ""
    EXIT_CODE=1
else
    echo "✅ PASS: All filenames follow conventions"
fi

echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Summary
# ─────────────────────────────────────────────────────────────────────────────

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ All migration validations passed!"
else
    echo "❌ Migration validation failed. Please fix the issues above."
fi

exit $EXIT_CODE
