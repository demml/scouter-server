INSERT INTO $table (name, repository, version, profile, cron, next_run) 
VALUES ('$name', '$repository', '$version', '$profile', '$cron', '$next_run')
ON CONFLICT DO NOTHING;