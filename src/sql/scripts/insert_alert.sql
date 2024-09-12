-- insert alerts into scouter.alerts
INSERT INTO $table (name, repository, version, alert)
VALUES ('$name', '$repository', '$version', '$alert')
ON CONFLICT DO NOTHING;