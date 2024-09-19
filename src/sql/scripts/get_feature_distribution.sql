WITH subquery AS (
    SELECT
    date_bin('$bin minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    feature,
    version,
    value
    from $table
    WHERE 
        created_at > timezone('utc', now()) - interval '$time_window' minute
        AND version = '$version'
        AND name = '$name'
        AND repository = '$repository'
        AND feature = '$feature'
),

aggregate AS (
    SELECT
        created_at,
        name,
        repository,
        feature,
        version,
        AVG(value) AS value
    FROM subquery
    GROUP BY
        created_at,
        name,
        repository,
        feature,
        version
),

percentiles AS (
    SELECT
        name,
        repository,
        feature,
        version,
        COUNT(1)::float8 as cnt,
        PERCENTILE_CONT(0.1) WITHIN GROUP (ORDER BY value)::float8 AS percentile_10,
        PERCENTILE_CONT(0.2) WITHIN GROUP (ORDER BY value)::float8 AS percentile_20,
        PERCENTILE_CONT(0.3) WITHIN GROUP (ORDER BY value)::float8 AS percentile_30,
        PERCENTILE_CONT(0.4) WITHIN GROUP (ORDER BY value)::float8 AS percentile_40,
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY value)::float8 AS percentile_50,
        PERCENTILE_CONT(0.6) WITHIN GROUP (ORDER BY value)::float8 AS percentile_60,
        PERCENTILE_CONT(0.7) WITHIN GROUP (ORDER BY value)::float8 AS percentile_70,
        PERCENTILE_CONT(0.8) WITHIN GROUP (ORDER BY value)::float8 AS percentile_80,
        PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY value)::float8 AS percentile_90,
        PERCENTILE_CONT(1) WITHIN GROUP (ORDER BY value)::float8 AS percentile_100
    FROM aggregate
    GROUP BY
        name,
        repository,
        feature,
        version
)

select
A.name,
A.repository,
A.Feature,
A.version,
percentile_10,
percentile_20,
percentile_30,
percentile_40,
percentile_50,
percentile_60,
percentile_70,
percentile_80,
percentile_90,
percentile_100,
SUM(case when A.VALUE <= percentile_10 then 1 else 0 end) / cnt as val_10,
SUM(case when (A.VALUE > percentile_10) and (A.VALUE <= percentile_20) then 1 else 0 end) / cnt as val_20,
SUM(case when (A.VALUE > percentile_20) and (A.VALUE <= percentile_30) then 1 else 0 end) / cnt  as val_30,
SUM(case when (A.VALUE > percentile_30) and (A.VALUE <= percentile_40) then 1 else 0 end) / cnt  as val_40,
SUM(case when (A.VALUE > percentile_40) and (A.VALUE <= percentile_50) then 1 else 0 end) / cnt  as val_50,
SUM(case when (A.VALUE > percentile_50) and (A.VALUE <= percentile_60) then 1 else 0 end) / cnt  as val_60,
SUM(case when (A.VALUE > percentile_60) and (A.VALUE <= percentile_70) then 1 else 0 end) / cnt  as val_70,
SUM(case when (A.VALUE > percentile_70) and (A.VALUE <= percentile_80) then 1 else 0 end) / cnt  as val_80,
SUM(case when (A.VALUE > percentile_80) and (A.VALUE <= percentile_90) then 1 else 0 end) / cnt  as val_90,
SUM(case when (A.VALUE > percentile_90) and (A.VALUE <= percentile_100) then 1 else 0 end) / cnt  as val_100
from aggregate as A
left join PERCENTILES as b 
	on A.NAME = B.NAME
	and A.REPOSITORY = B.REPOSITORY
	and A.version = B.version
	and A.FEATURE = B.FEATURE
group by 
	A.name,
	A.repository,
	A.Feature,
	A.version,
	b.cnt,
	percentile_10,
	percentile_20,
	percentile_30,
	percentile_40,
	percentile_50,
	percentile_60,
	percentile_70,
	percentile_80,
	percentile_90,
	percentile_100;