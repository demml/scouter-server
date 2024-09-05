INSERT INTO $table (name, repository, version, profile, schedule, next_run)
VALUES ('$name', '$repository', '$version', '$profile', '$schedule', '$next_run')
ON CONFLICT DO NOTHING;