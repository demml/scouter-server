SELECT
DISTINCT feature
FROM $table
WHERE
   name = '$name'
   AND repository = '$repository'
   AND version = '$version';