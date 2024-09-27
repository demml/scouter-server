with subquery as (
    SELECT
    date_bin('$bin minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    version,
    1 as alert,
    case when status = 'active' then 1 else 0 end active,
    case when status = 'acknowledged' then 1 else 0 end acknowledged
    from $table
    WHERE 
        created_at > timezone('utc', now()) - interval '$time_window' minute
        AND version = '$version'
        AND name = '$name'
        AND repository = '$repository'
)

SELECT
created_at,
name,
repository,
sum(alert) as alert_count,
sum(active) as active,
sum(acknowledged) as acknowledged
FROM subquery
GROUP BY 
    created_at,
    name,
    repository,
    version
ORDER BY
    created_at ASC;