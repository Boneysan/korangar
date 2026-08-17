#!/usr/bin/env bash
# Regression tests for run-integration-tests.sh's orphan sweep.
#
# The sweep deletes files and drops databases, so the case that matters most is
# the one where it must do NOTHING: a config a developer wrote by hand must
# never be mistaken for this runner's litter. Both directions are asserted here.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="$here/run-integration-tests.sh"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/korangar-orphan.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

failures=0
fail() {
    echo "  FAIL: $*" >&2
    failures=$((failures + 1))
}
ok() { echo "  ok: $*"; }

# A fake Hercules checkout is enough: the sweep only ever looks at conf/import.
fake_hercules="$test_root/Hercules"
mkdir -p "$fake_hercules/.git" "$fake_hercules/conf/import"

# Source the runner for its functions only. INTEGRATION_DB_ADMIN points at a
# user that does not exist so an accidental database call cannot touch a real
# server; every assertion here is about files.
INTEGRATION_RUNNER_SOURCE_ONLY=1 \
    HERCULES_DIR="$fake_hercules" \
    INTEGRATION_DB_ADMIN="korangar_orphan_test_nonexistent" \
    . "$runner"

echo "static_override reproduces each managed config"
for name in login-server char-server map-server inter-server; do
    static_override "$name" > "$test_root/$name.generated"
    [ -s "$test_root/$name.generated" ] || fail "$name generated an empty config"
done
ok "all four are non-empty"

echo "is_our_artifact recognises this runner's own leftovers"
for name in login-server char-server map-server inter-server; do
    target="$fake_hercules/conf/import/$name.conf"
    static_override "$name" > "$target"
    if is_our_artifact "$name" "$target"; then
        ok "$name identified as an artifact"
    else
        fail "$name was not recognised as this runner's own output"
    fi
done

cat > "$fake_hercules/conf/import/integration-sql.conf" <<'EOF'
sql_connection: {
    db_hostname: "127.0.0.1"
    db_port: 3306
    db_username: "korangar_int"
    db_password: "korangar_int"
    db_database: "korangar_integration_9784"
}
EOF
if is_our_artifact integration-sql "$fake_hercules/conf/import/integration-sql.conf"; then
    ok "integration-sql identified by the database namespace we own"
else
    fail "a real leaked integration-sql.conf was not recognised"
fi

echo "is_our_artifact leaves a developer's own config alone"
# A hand-written map-server.conf that happens to live at the same path.
cat > "$test_root/handwritten-map-server.conf" <<'EOF'
map_configuration: {
    // Local experiment: quieter logs while working on skill units.
    console_msg_log: 8
}
EOF
if is_our_artifact map-server "$test_root/handwritten-map-server.conf"; then
    fail "a hand-written map-server.conf was misidentified as runner litter"
else
    ok "hand-written map-server.conf correctly left alone"
fi

# The nastiest near-miss: a developer's own sql override, pointing at the real
# database. Deleting this would silently repoint someone's server.
cat > "$test_root/handwritten-sql.conf" <<'EOF'
sql_connection: {
    db_hostname: "127.0.0.1"
    db_database: "ragnarok"
}
EOF
if is_our_artifact integration-sql "$test_root/handwritten-sql.conf"; then
    fail "a developer's own sql_connection override was misidentified as litter"
else
    ok "sql override naming a non-disposable database left alone"
fi

# One edited line must be enough to make it not ours.
static_override char-server | sed 's/delay: 0/delay: 60/' > "$test_root/edited-char-server.conf"
if is_our_artifact char-server "$test_root/edited-char-server.conf"; then
    fail "an edited copy of our config was still claimed as ours"
else
    ok "an edited copy is no longer claimed"
fi

echo "reclaim_orphans restores Hercules' templates rather than deleting"
# A missing conf/import/*.conf is an [Error] on every server start, so the sweep
# must put the empty template back where upstream ships one.
mkdir -p "$fake_hercules/conf/import-tmpl"
for name in login-server char-server map-server inter-server; do
    printf '%s_configuration: {\n}\n' "$(echo "$name" | tr -d -)" > "$fake_hercules/conf/import-tmpl/$name.conf"
done
reclaim_orphans >/dev/null 2>&1 || true
for name in login-server char-server map-server inter-server; do
    target="$fake_hercules/conf/import/$name.conf"
    if [ ! -f "$target" ]; then
        fail "$name.conf was deleted instead of restored from the template"
    elif is_our_artifact "$name" "$target"; then
        fail "$name.conf still holds the runner's override after the sweep"
    else
        ok "$name.conf restored from the template"
    fi
done
# integration-sql.conf is entirely ours and has no template, so it goes.
if [ -f "$fake_hercules/conf/import/integration-sql.conf" ]; then
    fail "integration-sql.conf survived the sweep"
else
    ok "integration-sql.conf removed (no upstream template exists)"
fi

echo "reclaim_orphans preserves a developer's config in the same directory"
cp "$test_root/handwritten-map-server.conf" "$fake_hercules/conf/import/map-server.conf"
static_override login-server > "$fake_hercules/conf/import/login-server.conf"
reclaim_orphans >/dev/null 2>&1 || true
if [ -f "$fake_hercules/conf/import/map-server.conf" ]; then
    ok "hand-written map-server.conf survived a sweep that ran beside it"
else
    fail "the sweep deleted a hand-written config"
fi
if is_our_artifact login-server "$fake_hercules/conf/import/login-server.conf"; then
    fail "the artifact beside it was not reclaimed"
else
    ok "the artifact beside it was still reclaimed"
fi

if [ "$failures" -ne 0 ]; then
    echo "orphan-reclaim tests: $failures failure(s)" >&2
    exit 1
fi
echo "orphan-reclaim tests: all passed"
