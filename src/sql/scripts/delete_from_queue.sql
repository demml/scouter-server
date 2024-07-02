-- delete from queue

DELETE FROM $table
WHERE (
    SELECT 
    FROM $table
    WHERE
        $table.name = $name
        AND $table.repository = $repository
        AND $table.version = $version
        AND $table.next_run < $next_run
    ORDER BY $table.next_run
    FOR UPDATE SKIP LOCKED
    LIMIT  1
)
RETURNING *;
