WITH interval_series AS (

select 
	date_bin(($1 || 'minutes')::interval, created_at, TIMESTAMP '1970-01-01') as created_at
	from(
     SELECT generate_series(
        timezone('utc', now()) - (interval '1 minute' * $2),
        timezone('utc', now()),
        ($1 || ' minutes')::interval
    ) AS created_at
    )
),

subquery1 AS (
    SELECT
        date_bin(($1 || 'minutes')::interval, created_at, TIMESTAMP '1970-01-01') as created_at,
        jsonb_array_elements(route_metrics) as route_metric
    FROM scouter.observability_metrics
    WHERE 
        created_at > timezone('utc', now()) - (interval '1 minute' * $2)
        AND name = $3
        AND repository = $4
        AND version = $5
),

subquery2 AS (
    SELECT
        created_at,
        route_metric->>'route_name' as route_name,
        (route_metric->'metrics'->>'p5')::float as p5,
        (route_metric->'metrics'->>'p25')::float as p25,
        (route_metric->'metrics'->>'p50')::float as p50,
        (route_metric->'metrics'->>'p95')::float as p95,
        (route_metric->'metrics'->>'p99')::float as p99,
        (route_metric->>'request_count')::float as request_count,
        (route_metric->>'error_count')::float as error_count,
        (route_metric->>'error_latency')::float as error_latency,
        route_metric->'status_codes' as status_codes,
        30 as seconds
	FROM subquery1
),

route_series AS (
    SELECT
        r.route_name,
        i.created_at
    FROM (
    	select distinct route_name
    	from subquery2
    ) r
    CROSS JOIN interval_series i
),

expanded_status_codes as (

SELECT 
	created_at,
	route_name,
	jsonb_object_agg(status, count) as aggregated_map
from (

	select 
		created_at,
		route_name,
		(status_code).key as status,
		sum(((status_code).value::text)::integer) as count
	from (
		select 
			created_at,
			route_name,
			jsonb_each_text(status_codes) as status_code
			
		from subquery2
		)
	group by
		created_at,
		route_name,
		(status_code).key
	)
group by
		created_at,
		route_name
),

subquery3 AS (
    SELECT
        created_at,
        route_name,
        avg(p5) as avg_p5,
        avg(p25) as avg_p25,
        avg(p50) as avg_p50,
        avg(p95) as avg_p95,
        avg(p99) as avg_p99,
        sum(request_count) as total_request_count,
        sum(error_count) as total_error_count,
        avg(error_latency) as avg_error_latency,
        sum(seconds) as total_seconds
    FROM subquery2
    GROUP BY 
        created_at,
        route_name
),

joined AS (
    SELECT
        rs.created_at,
        rs.route_name,
        COALESCE(s3.avg_p5, 0) as avg_p5,
        COALESCE(s3.avg_p25, 0) as avg_p25,
        COALESCE(s3.avg_p50, 0) as avg_p50,
        COALESCE(s3.avg_p95, 0) as avg_p95,
        COALESCE(s3.avg_p99, 0) as avg_p99,
        COALESCE(s3.total_request_count, 0) as total_request_count,
        COALESCE(s3.total_error_count, 0) as total_error_count,
        CASE COALESCE(s3.total_request_count, 0) WHEN 0 THEN 0 ELSE COALESCE(s3.total_request_count, 0) / COALESCE(s3.total_seconds, 1) END as request_per_sec,
        CASE COALESCE(s3.total_error_count, 0) WHEN 0 THEN 0 ELSE COALESCE(s3.total_error_count, 0) / COALESCE(s3.total_seconds, 1) END as error_per_sec,
        COALESCE(s3.avg_error_latency, 0) as avg_error_latency,
        COALESCE(s3.total_seconds, 0) as total_seconds,
        COALESCE(es.aggregated_map, '{}'::jsonb) as status_counts
    FROM route_series rs
    LEFT JOIN subquery3 s3
        ON rs.created_at = s3.created_at
        AND rs.route_name = s3.route_name
    LEFT JOIN expanded_status_codes es
        ON rs.created_at = es.created_at
        AND rs.route_name = es.route_name
)

SELECT
    route_name,
    CASE SUM(total_request_count) WHEN 0 THEN 0 ELSE SUM(total_request_count) / SUM(total_seconds) END as total_request_per_sec,
    CASE SUM(total_error_count) WHEN 0 THEN 0 ELSE SUM(total_error_count) / SUM(total_seconds) END as total_error_per_sec,
    SUM(total_request_count)::bigint as total_request_count,
    SUM(total_error_count)::bigint as total_error_count,
    array_agg(created_at ORDER BY created_at ASC) as created_at,
    array_agg(avg_p5 ORDER BY created_at ASC) as p5,
    array_agg(avg_p25 ORDER BY created_at ASC) as p25,
    array_agg(avg_p50 ORDER BY created_at ASC) as p50,
    array_agg(avg_p95 ORDER BY created_at ASC) as p95,
    array_agg(avg_p99 ORDER BY created_at ASC) as p99,
    array_agg(request_per_sec ORDER BY created_at ASC) as request_per_sec,
    array_agg(error_per_sec ORDER BY created_at ASC) as error_per_sec,
    array_agg(avg_error_latency ORDER BY created_at ASC) as error_latency,
    array_agg(status_counts ORDER BY created_at ASC) as status_counts
FROM joined
GROUP BY 
    route_name;