-- update drift profile given name, repository and version

UPDATE $table
SET profile = '$profile'
WHERE name = '$name'
  and repository = '$repository'
  and version = '$version';