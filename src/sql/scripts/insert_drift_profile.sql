INSERT INTO $table (name, repository, version, profile, schedule, next_run, previous_run)
VALUES ('$name', '$repository', '$version', '$profile', '$schedule', '$next_run', '$previous_run')
ON CONFLICT DO NOTHING;