SELECT
created_at,
name,
repository,
version,
alert
FROM $table
WHERE
    version = '$version'
    AND name = '$name'
    AND repository = '$repository'