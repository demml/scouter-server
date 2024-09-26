-- updates drift alert status for a given id

UPDATE $table
SET status = '$status'
WHERE id = $id;