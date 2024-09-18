with subquery as (
    SELECT
    date_bin('$bin minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    feature,
    version,
    value
    from $table
    WHERE 
        created_at > timezone('utc', now()) - interval '$time_window' minute
        AND version = '$version'
        AND name = '$name'
        AND repository = '$repository'
        AND feature = '$feature'
)

SELECT
created_at,
name,
repository,
feature,
version,
avg(value) as value
FROM subquery
GROUP BY 
    created_at,
    name,
    repository,
    feature,
    version
ORDER BY
    created_at ASC;