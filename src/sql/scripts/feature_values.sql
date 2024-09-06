SELECT
created_at,
feature,
value
FROM $table
WHERE
    created_at > '$limit_timestamp'
    AND version = '$version'
    AND name = '$name'
    AND repository = '$repository'
    AND feature = '$feature';