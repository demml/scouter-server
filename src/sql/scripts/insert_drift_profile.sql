INSERT INTO $table (name, repository, version, profile, active, schedule, next_run, previous_run)
VALUES ('$name', '$repository', '$version', '$profile', '$active', '$schedule', '$next_run', '$previous_run')
ON CONFLICT DO NOTHING;