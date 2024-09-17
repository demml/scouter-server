INSERT INTO $table (name, repository, version, scouter_version, profile, active, schedule, next_run, previous_run)
VALUES ('$name', '$repository', '$version', '$scouter_version', '$profile', '$active', '$schedule', '$next_run', '$previous_run')
ON CONFLICT DO NOTHING;