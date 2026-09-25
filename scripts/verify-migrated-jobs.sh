#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Compares Kaaryasoochi's run records for a day with the scripts' own cron.log entries, to
# check the migrated vishleshak jobs behave like the crontab entries they mirror.
#
#   scripts/verify-migrated-jobs.sh [YYYY-MM-DD]     (default: today, local time)
#
# Exit status: 0 = every expected run happened and succeeded, 1 = something is off.
set -uo pipefail

DAY="${1:-$(date +%F)}"
DB="${KAARYASOOCHI_DB:-$HOME/.local/share/kaaryasoochi/kaaryasoochi.db}"
REPO="${VISHLESHAK_DIR:-$HOME/work/mission_samruddhi/vishleshak}"
EXPECTED="${EXPECTED_RUNS:-3}" # three migrated jobs, each once per weekday

echo "== Kaaryasoochi runs on $DAY (local time)"
sqlite3 -readonly -header -column "$DB" "
  SELECT j.title,
         datetime(r.scheduled_for,'localtime') AS scheduled,
         datetime(r.started_at,'localtime')    AS started,
         datetime(r.finished_at,'localtime')   AS finished,
         r.status, r.exit_code AS exit
  FROM job_run r JOIN job j ON j.id = r.job_id
  WHERE date(r.started_at,'localtime') = '$DAY' AND j.title LIKE 'Vishleshak:%'
  ORDER BY r.started_at;"

echo
echo "== cron.log entries on $DAY (crontab at :00/:05, Kaaryasoochi one minute later)"
for log in data/bhav_copy_archive/cron.log \
           data/reports/strategies/zerodha_ta_tutorial/cron.log \
           data/reports/cron.log; do
  echo "-- $log"
  grep -h "^=== $DAY" "$REPO/$log" 2>/dev/null || echo "   (no entries)"
done

total=$(sqlite3 -readonly "$DB" "SELECT count(*) FROM job_run r JOIN job j ON j.id=r.job_id
  WHERE date(r.started_at,'localtime')='$DAY' AND j.title LIKE 'Vishleshak:%';")
ok=$(sqlite3 -readonly "$DB" "SELECT count(*) FROM job_run r JOIN job j ON j.id=r.job_id
  WHERE date(r.started_at,'localtime')='$DAY' AND j.title LIKE 'Vishleshak:%'
  AND r.status='success' AND r.exit_code=0;")
echo
if [[ "$total" -eq "$EXPECTED" && "$ok" -eq "$EXPECTED" ]]; then
  echo "PASS: $ok/$EXPECTED runs succeeded with exit code 0"
else
  echo "FAIL: expected $EXPECTED successful runs, found $total run(s), $ok successful" >&2
  exit 1
fi
