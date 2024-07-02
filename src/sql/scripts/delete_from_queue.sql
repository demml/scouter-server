-- delete from queue

DELETE FROM $table
WHERE EXISTS (
    SELECT 1
    FROM $table
    WHERE
        $table.name = $name
        AND $table.repository = $repository
        AND $table.version = $version
        AND $table.next_run < $next_run
);
