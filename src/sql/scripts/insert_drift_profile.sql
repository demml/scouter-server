INSERT INTO scouter.drift_profile (name, repository, version, scouter_version, profile, active, schedule, next_run, previous_run)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT DO NOTHING;