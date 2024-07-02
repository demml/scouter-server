INSERT INTO $table (name, repository, version, next_run) 
VALUES ('$name', '$repository', '$version', '$next_run')
ON CONFLICT DO NOTHING;