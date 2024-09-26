SELECT
created_at,
name,
repository,
version,
feature,
alert,
id,
status
FROM $table
WHERE
    version = '$version'
    AND name = '$name'
    AND repository = '$repository'