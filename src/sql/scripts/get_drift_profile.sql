with PROFILE as (SELECT name, repository, version
                 FROM $table
                 WHERE active
                   AND next_run < CURRENT_TIMESTAMP
                   AND NOT processing
                 LIMIT 1 FOR UPDATE SKIP LOCKED)

UPDATE $table
SET processing = true
WHERE (name, repository, version) IN (SELECT name, repository, version from PROFILE)
RETURNING profile, previous_run, schedule;