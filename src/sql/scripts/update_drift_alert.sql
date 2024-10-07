-- updates drift alert status for a given id

UPDATE scouter.drift_alerts
SET status = $1
WHERE id = $2;