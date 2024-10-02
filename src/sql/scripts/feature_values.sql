SELECT
created_at,
feature,
value
FROM scouter.drift
WHERE
    created_at > $1::timestamp
    AND name = $2
    AND repository = $3
    AND version = $4
    AND feature = $5;