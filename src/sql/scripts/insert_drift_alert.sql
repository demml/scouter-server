-- insert alerts into scouter.alerts
INSERT INTO $table (name, repository, version, feature, alert)
VALUES ('$name', '$repository', '$version', '$feature', '$alert')
ON CONFLICT DO NOTHING;