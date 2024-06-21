INSERT INTO $table (created_at, name, repository, version, feature, value) 
VALUES ('$created_at', '$name', '$repository', '$version', '$feature', '$value')
ON CONFLICT DO NOTHING;