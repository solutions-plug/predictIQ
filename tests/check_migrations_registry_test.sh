#!/usr/bin/env bash
# Tests scripts/check_migrations_registry.sh against temp fixtures.
set -uo pipefail
SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/scripts/check_migrations_registry.sh"
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
failures=0

make_fixture() { # $1 = dir
  mkdir -p "$1/services/api/database/migrations/rollbacks" "$1/services/api/src"
  : > "$1/services/api/src/migrations.rs"
}
add_migration() { # $1 dir, $2 name, $3 version
  local d="$1/services/api/database/migrations"
  : > "$d/$2.sql"; : > "$d/rollbacks/$2_down.sql"
  echo "Migration { version: \"$3\", name: \"$2\" }," >> "$1/services/api/src/migrations.rs"
}
expect() { # $1 label, $2 expected zero|nonzero, $3 dir
  (cd "$3" && bash "$SCRIPT" >/dev/null 2>&1); local rc=$?
  if { [ "$2" = zero ] && [ $rc -eq 0 ]; } || { [ "$2" = nonzero ] && [ $rc -ne 0 ]; }; then
    echo "PASS: $1"
  else
    echo "FAIL: $1 (exit $rc)"; failures=$((failures+1))
  fi
}

d="$tmp/clean"; make_fixture "$d"
add_migration "$d" 001_init 001; add_migration "$d" 002_users 002
expect "clean fixture exits zero" zero "$d"

d="$tmp/dup"; make_fixture "$d"
add_migration "$d" 001_init 001; add_migration "$d" 001_other 001
expect "duplicate version prefix exits non-zero" nonzero "$d"

d="$tmp/norollback"; make_fixture "$d"
add_migration "$d" 001_init 001; rm "$d/services/api/database/migrations/rollbacks/001_init_down.sql"
expect "missing rollback exits non-zero" nonzero "$d"

d="$tmp/unregistered"; make_fixture "$d"
add_migration "$d" 001_init 001; : > "$d/services/api/database/migrations/002_x.sql"
expect "unregistered migration exits non-zero" nonzero "$d"

exit $failures
