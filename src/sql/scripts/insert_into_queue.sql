DO
$do$
BEGIN
   IF NOT EXISTS (
        SELECT * FROM $table
        WHERE name = '$name'
        AND repository = '$repository'
        AND version = '$version'
        AND next_run = '$next_run'
        ) THEN
      DELETE FROM orders;
      INSERT INTO $table (name, repository, version, next_run) VALUES ('$name', '$repository', '$version', '$next_run');
   END IF;
END
$do$