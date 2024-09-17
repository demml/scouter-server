SELECT profile
FROM $table
WHERE name = '$name'
  and repository = '$repository'
  and version = '$version';