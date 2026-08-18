#!/usr/bin/env bash
# GUI-pass pre-flight: report the two seats, from the database the servers are
# ACTUALLY using.
#
# Why this exists. On 2026-08-16 a live session was brought up against a
# leftover `korangar_integration_*` database: a killed run had left the import
# overrides installed in Hercules/conf/import, so every server silently pointed
# at a disposable database. The seats were read out of `ragnarok` and looked
# fine, while the characters on screen were two Novices from a fixture. Nothing
# in that session would have counted.
#
# So this never just queries a database — it first works out which one the
# running servers are connected to, and says so on every line of output. An
# unnamed answer is what caused the incident.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
client_repo="$(cd "$here/../.." && pwd)"
hercules_repo="${HERCULES_DIR:-$(cd "$client_repo/../Hercules" 2>/dev/null && pwd || true)}"

if [ -z "$hercules_repo" ] || [ ! -d "$hercules_repo" ]; then
    echo "error: Hercules checkout not found; set HERCULES_DIR" >&2
    exit 2
fi

sql_conf="$hercules_repo/conf/global/sql_connection.conf"
import_dir="$hercules_repo/conf/import"

sqlval() {
    sed -n "s/^[[:space:]]*$1:[[:space:]]*\"\(.*\)\".*/\1/p" "$2" | head -1
}

problems=0
note_problem() {
    echo "  !! $*"
    problems=$((problems + 1))
}

echo "== which database =="

# 1. What the config says, honouring an import override the way Hercules does.
configured_db="$(sqlval db_database "$sql_conf")"
config_source="conf/global/sql_connection.conf"
if [ -f "$import_dir/integration-sql.conf" ]; then
    configured_db="$(sqlval db_database "$import_dir/integration-sql.conf")"
    config_source="conf/import/integration-sql.conf (OVERRIDE)"
fi
db_host="$(sqlval db_hostname "$sql_conf")"
db_user="$(sqlval db_username "$sql_conf")"
db_pass="$(sqlval db_password "$sql_conf")"
echo "  config says:  $configured_db   [$config_source]"

# 2. An integration override present outside a run is the incident's signature.
if [ -f "$import_dir/integration-sql.conf" ]; then
    note_problem "an integration override is installed in conf/import."
    echo "     A killed integration run left it behind. Any session started now"
    echo "     runs against a disposable database. Remove the five files"
    echo "     (login/char/map/inter-server.conf + integration-sql.conf), or run"
    echo "     tools/testing/run-integration-tests.sh once — it now reclaims them."
fi
case "$configured_db" in
korangar_integration_*)
    note_problem "the effective database is a disposable integration database."
    ;;
esac

mysql_cmd=(mysql --protocol=tcp --host="${db_host:-127.0.0.1}" --user="$db_user" "--password=$db_pass")

# 3. Ground truth: what the running servers actually hold connections to. This
#    is the check that would have caught the incident in seconds, because it
#    cannot be fooled by reading the same config the report is based on.
live_dbs="$("${mysql_cmd[@]}" --batch --skip-column-names --execute="
    SELECT DISTINCT db FROM information_schema.processlist
     WHERE db IS NOT NULL AND user = '$db_user';" 2>/dev/null || true)"

if [ -z "$live_dbs" ]; then
    echo "  servers:      no live server connections found — is Hercules running?"
    echo "                (reporting against the configured database instead)"
    effective_db="$configured_db"
else
    echo "  servers use:  $(echo "$live_dbs" | tr '\n' ' ')"
    effective_db="$(echo "$live_dbs" | head -1)"
    if [ "$(echo "$live_dbs" | wc -l | tr -d ' ')" != 1 ]; then
        note_problem "servers are connected to more than one database."
    elif [ "$effective_db" != "$configured_db" ]; then
        note_problem "the servers are on '$effective_db' but the config now reads '$configured_db'."
        echo "     The servers are running against a database the config no longer"
        echo "     names — they were started before it changed. Restart Hercules."
    fi
fi

echo
echo "== seats, read from '$effective_db' =="

# The database goes in as an argument rather than qualifying each table: the
# subquery on `inventory` would otherwise need its own qualifier, and one
# unqualified table is enough to fail the whole report with "No database
# selected".
seat_report="$("${mysql_cmd[@]}" "$effective_db" --table --execute="
SELECT c.name                                        AS seat,
       c.class,
       CASE c.class WHEN 4020 THEN 'Clown  OK'
                    WHEN 4021 THEN 'Gypsy  OK'
                    ELSE CONCAT('WRONG JOB (want ',
                         CASE c.name WHEN 'test' THEN '4020 Clown' ELSE '4021 Gypsy' END, ')')
       END                                           AS job_check,
       c.last_map,
       CONCAT(c.last_x, ',', c.last_y)               AS pos,
       c.party_id,
       COALESCE((SELECT CASE WHEN i.equip > 0 THEN 'equipped' ELSE 'in bag, NOT equipped' END
                   FROM inventory i
                  WHERE i.char_id = c.char_id
                    AND i.nameid = CASE c.name WHEN 'test' THEN 1901 ELSE 1950 END
                  LIMIT 1), 'MISSING')               AS instrument
  FROM \`char\` c
 WHERE c.name IN ('test', 'HeadlessTwo')
 ORDER BY c.name;" 2>&1)" || {
    echo "  could not read the seats from '$effective_db'" >&2
    exit 1
}

if ! echo "$seat_report" | grep -q .; then
    note_problem "neither seat exists in '$effective_db'."
else
    echo "$seat_report"
fi

if ! echo "$seat_report" | grep -q 'test'; then
    note_problem "seat A ('test') does not exist in '$effective_db'."
fi
if ! echo "$seat_report" | grep -q 'HeadlessTwo'; then
    note_problem "seat B ('HeadlessTwo') does not exist in '$effective_db'."
fi

party_count="$("${mysql_cmd[@]}" "$effective_db" --batch --skip-column-names --execute="
    SELECT COUNT(*) FROM party;" 2>/dev/null || echo 0)"
echo
echo "  parties in '$effective_db': $party_count"
[ "$party_count" = 0 ] && echo "  (§1 Hermode needs both seats in one party — create it in the client)"

echo
# The `char` table is only as fresh as the last save, so a green report is not a
# statement about a session already in progress.
echo "note: \`char\` reflects the last save, not live state. Run this BEFORE"
echo "      logging in, or after a relog, not to check a running session."

if [ "$problems" -ne 0 ]; then
    echo
    echo "pre-flight: $problems problem(s) above — fix before testing." >&2
    exit 1
fi
echo
echo "pre-flight: seats readable from '$effective_db'; check the job/instrument columns above."
