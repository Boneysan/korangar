#!/usr/bin/env bash
# Disposable Korangar + Hercules integration runner.
#
# Creates a unique database, temporarily installs ignored Hercules import
# configs, starts only the three protocol servers the headless client needs,
# runs the requested scenarios, audits fixture cleanup, and tears everything
# down on success, failure, or interruption.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
client_repo="$(cd "$here/../.." && pwd)"
hercules_repo="${HERCULES_DIR:-$(cd "$client_repo/../Hercules" 2>/dev/null && pwd || true)}"

if [ -z "$hercules_repo" ] || [ ! -d "$hercules_repo/.git" ]; then
    echo "error: Hercules checkout not found; set HERCULES_DIR" >&2
    exit 2
fi

for command_name in mysql nc cargo git; do
    command -v "$command_name" >/dev/null 2>&1 || {
        echo "error: required command not found: $command_name" >&2
        exit 2
    }
done

db_host="${INTEGRATION_DB_HOST:-127.0.0.1}"
db_port="${INTEGRATION_DB_PORT:-3306}"
db_admin="${INTEGRATION_DB_ADMIN:-root}"
db_password="${INTEGRATION_DB_ADMIN_PASSWORD:-}"
db_name="korangar_integration_$$"
ready_timeout="${INTEGRATION_READY_TIMEOUT:-900}"
build_jobs="${INTEGRATION_BUILD_JOBS:-2}"
scratch="$(mktemp -d)"
server_log_dir="$client_repo/target/headless-server-logs"
mkdir -p "$server_log_dir"
# Where the results land, so the artifact hint on failure names the real file
# rather than a default the caller may have overridden. Resolved properly beside
# the suite invocation; this is the value an early failure reports.
results_json="$client_repo/target/headless-results.json"

for numeric_value in "$db_port" "$ready_timeout" "$build_jobs"; do
    case "$numeric_value" in
    '' | 0 | *[!0-9]*)
        echo "error: DB port, readiness timeout, and build jobs must be positive integers" >&2
        exit 2
        ;;
    esac
done
case "$db_host$db_admin$db_password" in
*\"* | *\\* | *'{'* | *'}'* | *$'\n'* | *$'\r'*)
    echo "error: database connection values contain characters unsafe for Hercules config" >&2
    exit 2
    ;;
esac

mysql_admin=(mysql --protocol=tcp --host="$db_host" --port="$db_port" --user="$db_admin")
if [ -n "$db_password" ]; then
    mysql_admin+=("--password=$db_password")
fi

server_pids=()
database_created=false
configs_installed=false
runner_exit=1

stop_servers() {
    local pid attempt still_running
    for pid in "${server_pids[@]-}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done

    # Hercules normally exits promptly on SIGTERM, but map-server can remain
    # alive indefinitely while tearing down a heavily exercised skill run.
    # Never strand CI after the tester has already produced its result: allow a
    # bounded graceful window, then stop only the exact PIDs this script owns.
    attempt=0
    while [ "$attempt" -lt 100 ]; do
        still_running=false
        for pid in "${server_pids[@]-}"; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                still_running=true
                break
            fi
        done
        ! $still_running && break
        sleep 0.1
        attempt=$((attempt + 1))
    done

    for pid in "${server_pids[@]-}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "warning: server pid $pid did not stop gracefully; forcing shutdown" >&2
            kill -9 "$pid" 2>/dev/null || true
        fi
        [ -n "$pid" ] && wait "$pid" 2>/dev/null || true
    done
    server_pids=()
}

restore_configs() {
    local name source target
    for name in login-server char-server map-server inter-server integration-sql; do
        source="$scratch/$name.conf"
        target="$hercules_repo/conf/import/$name.conf"
        if [ -f "$source" ]; then
            cp "$source" "$target"
        elif [ -f "$scratch/$name.missing" ]; then
            rm -f "$target"
        fi
    done
    configs_installed=false
}

cleanup() {
    local original_exit=$?
    stop_servers
    $configs_installed && restore_configs
    if $database_created; then
        "${mysql_admin[@]}" --execute="DROP DATABASE IF EXISTS \`$db_name\`;" >/dev/null 2>&1 || true
    fi
    rm -rf "$scratch"
    if [ "$runner_exit" -ne 0 ]; then
        echo "integration artifacts: $results_json and $server_log_dir" >&2
    fi
    return "$original_exit"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

for port in 6900 6121 5121; do
    if nc -z 127.0.0.1 "$port" 2>/dev/null; then
        echo "error: TCP port $port is already in use; refusing to test against an ambiguous server" >&2
        exit 2
    fi
done

echo "creating disposable database $db_name on $db_host:$db_port"
"${mysql_admin[@]}" --execute="CREATE DATABASE \`$db_name\` CHARACTER SET utf8mb4;"
database_created=true
"${mysql_admin[@]}" "$db_name" < "$hercules_repo/sql-files/main.sql"
"${mysql_admin[@]}" "$db_name" < "$hercules_repo/sql-files/logs.sql"

"${mysql_admin[@]}" "$db_name" <<'SQL'
-- Email must be `a@a.com` (or match DeleteCharacterPacket): Hercules rejects
-- deletion when the confirmation email does not match login.email, and the
-- headless client always sends a@a.com (see NetworkingSystem::delete_character).
INSERT INTO `login` (`account_id`, `userid`, `user_pass`, `sex`, `email`, `group_id`, `character_slots`)
VALUES
    (2000000, 'korangar', 'korangar', 'M', 'a@a.com', 99, 9),
    (2000001, 'headless2', 'headless2pw', 'F', 'a@a.com', 99, 9);

INSERT INTO `char`
    (`char_id`, `account_id`, `char_num`, `name`, `class`, `base_level`, `job_level`,
     `max_hp`, `hp`, `max_sp`, `sp`, `last_map`, `last_x`, `last_y`,
     `save_map`, `save_x`, `save_y`, `slotchange`, `sex`)
VALUES
    (150000, 2000000, 0, 'HeadlessOne', 0, 99, 10,
     1000, 1000, 100, 100, 'prontera', 155, 180, 'prontera', 155, 180, 0, 'M'),
    (150001, 2000001, 0, 'HeadlessTwo', 0, 99, 10,
     1000, 1000, 100, 100, 'prontera', 156, 180, 'prontera', 156, 180, 10000, 'F');
SQL

mkdir -p "$hercules_repo/conf/import"
for name in login-server char-server map-server inter-server integration-sql; do
    target="$hercules_repo/conf/import/$name.conf"
    if [ -f "$target" ]; then
        cp "$target" "$scratch/$name.conf"
    else
        : > "$scratch/$name.missing"
    fi
done
configs_installed=true

cat > "$hercules_repo/conf/import/integration-sql.conf" <<EOF
sql_connection: {
    db_hostname: "$db_host"
    db_port: $db_port
    db_username: "$db_admin"
    db_password: "$db_password"
    db_database: "$db_name"
}
EOF
cat > "$hercules_repo/conf/import/login-server.conf" <<'EOF'
login_configuration: {
    account: {
        @include "conf/import/integration-sql.conf"
        ipban: {
            @include "conf/import/integration-sql.conf"
        }
    }
}
EOF
cat > "$hercules_repo/conf/import/char-server.conf" <<'EOF'
char_configuration: {
    @include "conf/import/integration-sql.conf"
    // Headless lifecycle tests create and delete characters immediately.
    enable_char_creation: true
    player: {
        deletion: {
            delay: 0
            level: 0
        }
    }
}
EOF
cat > "$hercules_repo/conf/import/map-server.conf" <<'EOF'
map_configuration: {
    @include "conf/import/integration-sql.conf"
}
EOF
cat > "$hercules_repo/conf/import/inter-server.conf" <<'EOF'
inter_configuration: {
    log: {
        @include "conf/import/integration-sql.conf"
    }
}
EOF

if [ "${INTEGRATION_SKIP_BUILD:-0}" != "1" ]; then
    echo "building Hercules for PACKETVER 20220406"
    (
        cd "$hercules_repo"
        # Homebrew is intentionally outside clang's default header/library
        # search path on Apple Silicon. Hercules' configure has no --with-pcre
        # option, so make the installed PCRE visible without affecting Linux CI.
        if [ "$(uname -s)" = "Darwin" ] && command -v brew >/dev/null 2>&1; then
            pcre_prefix="$(brew --prefix pcre)"
            export CPPFLAGS="-I$pcre_prefix/include${CPPFLAGS:+ $CPPFLAGS}"
            export LDFLAGS="-L$pcre_prefix/lib${LDFLAGS:+ $LDFLAGS}"
        fi
        ./configure --enable-packetver=20220406
        make -j"$build_jobs"
    )
fi
for binary in login-server char-server map-server; do
    [ -x "$hercules_repo/$binary" ] || {
        echo "error: $hercules_repo/$binary is missing; build Hercules or unset INTEGRATION_SKIP_BUILD" >&2
        exit 2
    }
done

start_server() {
    local binary=$1
    (
        cd "$hercules_repo"
        exec "./$binary"
    ) > "$server_log_dir/$binary.log" 2>&1 &
    server_pids+=("$!")
}

echo "starting disposable Hercules stack"
start_server login-server
start_server char-server
start_server map-server

elapsed=0
while [ "$elapsed" -lt "$ready_timeout" ]; do
    if nc -z 127.0.0.1 6900 2>/dev/null \
        && nc -z 127.0.0.1 6121 2>/dev/null \
        && nc -z 127.0.0.1 5121 2>/dev/null \
        && grep -qF "listening on port '5121'" "$server_log_dir/map-server.log"; then
        break
    fi
    for pid in "${server_pids[@]}"; do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "error: a Hercules server died during startup" >&2
            tail -n 40 -- "$server_log_dir"/*.log >&2 || true
            exit 1
        fi
    done
    sleep 2
    elapsed=$((elapsed + 2))
done
if [ "$elapsed" -ge "$ready_timeout" ]; then
    echo "error: Hercules did not become ready within ${ready_timeout}s" >&2
    tail -n 40 -- "$server_log_dir"/*.log >&2 || true
    exit 1
fi

# Marked `-dirty` when the tree does not match the commit, because a bare SHA in
# an archive is a claim that the result is reproducible from it. Suite work is
# routinely run before it is committed — the 2026-08-12 full run was recorded
# against a commit that contained neither of the scenarios it exercised — and a
# stale-looking archive is worse than an honest one nobody can misread.
revision_suffix=""
git -C "$client_repo" diff --quiet HEAD 2>/dev/null || revision_suffix="-dirty"
export KORANGAR_CLIENT_REVISION="$(git -C "$client_repo" rev-parse HEAD)$revision_suffix"

revision_suffix=""
git -C "$hercules_repo" diff --quiet HEAD 2>/dev/null || revision_suffix="-dirty"
export KORANGAR_HERCULES_REVISION="$(git -C "$hercules_repo" rev-parse HEAD)$revision_suffix"

# `--fail-on-flaky` is the strict gate and is always on here, but a caller who
# passes it explicitly must not be punished for agreeing: the tester rejects a
# repeated flag ("cannot be used multiple times") and the run dies having
# already built Hercules and created the database, with a message about
# argument parsing. That cost two runs on 2026-08-11.
suite_arguments=("$@")
case " $* " in
*" --fail-on-flaky "*) ;;
*) suite_arguments+=(--fail-on-flaky) ;;
esac

previous_argument=""
for argument in "$@"; do
    [ "$previous_argument" = "--results-json" ] && results_json="$argument"
    previous_argument="$argument"
done
case " $* " in
*" --results-json "*) ;;
*) suite_arguments+=(--results-json "$results_json") ;;
esac

set +e
"$here/run-suite.sh" "${suite_arguments[@]}"
runner_exit=$?
set -e

# Stop first so online flags and character state are flushed before the audit.
stop_servers
sleep 1

fixture_violations=$("${mysql_admin[@]}" --batch --skip-column-names "$db_name" <<'SQL'
SELECT CONCAT('online characters: ', COUNT(*)) FROM `char` WHERE `online` <> 0 HAVING COUNT(*) <> 0;
SELECT CONCAT('temporary characters: ', COUNT(*)) FROM `char`
  WHERE `name` LIKE 'HlTmp%' OR `name` LIKE 'HlDel%' OR `name` LIKE 'HlSwap%'
  HAVING COUNT(*) <> 0;
SELECT CONCAT('characters still in a party: ', COUNT(*)) FROM `char` WHERE `party_id` <> 0 HAVING COUNT(*) <> 0;
SELECT CONCAT('party rows still present: ', COUNT(*)) FROM `party` HAVING COUNT(*) <> 0;
SELECT CONCAT('campaign globals still set: ', COUNT(*)) FROM `map_reg_num_db`
  WHERE (`key` LIKE '$dm\_%' OR `key` LIKE 'dm\_%') AND `value` <> 0 HAVING COUNT(*) <> 0;
SQL
)
if [ -n "$fixture_violations" ]; then
    echo "fixture cleanup audit failed:" >&2
    printf '  %s\n' "$fixture_violations" >&2
    runner_exit=1
else
    echo "fixture cleanup audit: clean"
fi

exit "$runner_exit"
