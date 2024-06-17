INSERT INTO $table 
(created_at, service_name, feature, value, version) 
SELECT * FROM  UNNEST(
    $created_at::timestamp[],
    $service_name::varchar[],
    $feature::varchar[],
    $value::double precision[],
    $version::varchar[]
);