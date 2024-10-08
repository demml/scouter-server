WITH subquery1 AS (
    SELECT
        date_bin('.05 minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
        name,
        repository,
        feature,
        version,
        value
    FROM scouter.drift
    WHERE 
        created_at > timezone('utc', now()) - interval '1 minute' * $1
        AND name = $2
        AND repository = $3
        AND version = $4
        AND feature = $5
),

subquery2 AS (
    SELECT
        created_at,
        name,
        repository,
        feature,
        version,
        avg(value) as value
    FROM subquery1
    GROUP BY 
        created_at,
        name,
        repository,
        feature,
        version
)

SELECT
    feature,
    array_agg(created_at ORDER BY created_at DESC) as created_at,
    array_agg(value ORDER BY created_at DESC) as values
FROM subquery2
GROUP BY 
    feature;
