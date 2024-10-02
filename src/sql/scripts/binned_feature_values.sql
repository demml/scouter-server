with subquery as (
    SELECT
    date_bin('$1 minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    feature,
    version,
    value
    from scouter.drift
    WHERE 
        created_at > timezone('utc', now()) - interval $2 minute
        AND name = $3
        AND repository = $4
        AND version = $5
        AND feature = $6
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
    created_at DESC;