UPDATE $table
SET active = '$active'
WHERE name = '$name'
  and repository = '$repository'
  and version = '$version';