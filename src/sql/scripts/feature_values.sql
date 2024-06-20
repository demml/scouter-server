SELECT
created_at,
feature,
value,
version
FROM $table
WHERE
    created_at > timezone('utc', now()) - interval '$time_window' minute
    AND version = '$version'
    AND name = '$name'
    AND repository = '$repository'
    AND feature = '$feature';