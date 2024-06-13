SELECT
created_at,
feature,
value,
version
FROM $table
WHERE
    created_at > timezone('utc', now()) - interval '$time_window' minute
    AND version = '$version'
    AND service_name = '$service_name'
    AND feature = '$feature';