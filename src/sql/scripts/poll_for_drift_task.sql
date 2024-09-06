SELECT name, repository, version, profile, previous_run, schedule
FROM $table
WHERE active
  AND next_run < CURRENT_TIMESTAMP
LIMIT 1 FOR UPDATE SKIP LOCKED;