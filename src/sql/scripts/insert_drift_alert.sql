-- insert alerts into scouter.alerts
INSERT INTO scouter.drift_alerts (name, repository, version, alert)
VALUES ($1, $2, $3, $4)
ON CONFLICT DO NOTHING;