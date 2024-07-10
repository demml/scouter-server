-- delete from queue

DELETE FROM $table
WHERE (
    SELECT *
    FROM $table
    ORDER BY next_run
    FOR UPDATE SKIP LOCKED
    LIMIT  1
)
RETURNING *;
