UPDATE $table
SET previous_run = next_run,
    next_run     = '$next_run',
    updated_at   = timezone('utc', now())
WHERE name = '$name'
  and repository = '$repository'
  and version = '$version';