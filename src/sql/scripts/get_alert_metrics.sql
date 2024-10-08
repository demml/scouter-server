with subquery as (
    SELECT
    date_bin(($1 || 'minutes')::interval, created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    version,
    1 as alert,
    case when status = 'active' then 1 else 0 end active,
    case when status = 'acknowledged' then 1 else 0 end acknowledged
    from scouter.drift_alerts
    WHERE 
        created_at > timezone('utc', now()) - (interval '1 minute' * $2)
        AND name = $3
        AND repository = $4
        AND version = $5
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