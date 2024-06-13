with subquery as (
    SELECT 
    date_bin('$bin minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    service_name,
    feature,
    version,
    value
    from $table
    WHERE 
        created_at > timezone('utc', now()) - interval '$time_window' minute
        AND version = '$version'
        AND service_name = '$service_name'
        AND feature = '$feature'
)

SELECT
created_at,
service_name,
feature,
version,
avg(value) as value
FROM subquery
GROUP BY 
    created_at,
    service_name,
    feature,
    version
ORDER BY
    created_at DESC;