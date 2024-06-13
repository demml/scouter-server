SELECT
DISTINCT feature
FROM $table
WHERE
   service_name = '$service_name'
   AND version = '$version';