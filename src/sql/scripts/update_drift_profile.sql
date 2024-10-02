-- update drift profile given name, repository and version

UPDATE scouter.drift_profile
SET profile = $1
WHERE name = $2
  and repository = $3
  and version = $4;