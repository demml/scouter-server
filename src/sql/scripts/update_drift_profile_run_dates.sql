UPDATE $table
SET next_run = CURRENT_TIMESTAMP + interval '1 minute',
    next_run     = '$next_run'
WHERE name = '$name'
  and repository = '$repository'
  and version = '$version';