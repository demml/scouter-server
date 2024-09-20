SELECT
created_at,
name,
repository,
version,
feature,
alert
FROM $table
WHERE
    version = '$version'
    AND name = '$name'
    AND repository = '$repository'