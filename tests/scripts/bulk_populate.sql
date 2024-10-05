CREATE EXTENSION IF NOT EXISTS tablefunc;

CREATE OR REPLACE FUNCTION insert_observability_data(num_records INTEGER)
RETURNS VOID AS $$
DECLARE
    start_time TIMESTAMP := timezone('utc', now());
    interval_seconds INTEGER := 2 * 60 * 60 / num_records; -- 2 hours divided by num_records
    i INTEGER;
BEGIN
    FOR i IN 0..(num_records - 1) LOOP
        INSERT INTO scouter.observability_metrics (
            created_at, repository, name, version, request_count, error_count, route_metrics
        ) VALUES (
            start_time + (i * interval_seconds) * INTERVAL '1 second',
            'example-repo-' || i,
            'example-service-' || i,
            '1.0.0',
            100 + i,
            5 + i,
            '[
                {
                    "route_name": "route1",
                    "metrics": {
                        "p5": 10.5,
                        "p25": 20.5,
                        "p50": 30.5,
                        "p95": 40.5,
                        "p99": 50.5
                    },
                    "request_count": 50,
                    "error_count": 2,
                    "error_latency": 5.5,
                    "status_codes": {
                        "200": 45,
                        "500": 2,
                        "404": 3
                    }
                },
                {
                    "route_name": "route2",
                    "metrics": {
                        "p5": 15.5,
                        "p25": 25.5,
                        "p50": 35.5,
                        "p95": 45.5,
                        "p99": 55.5
                    },
                    "request_count": 50,
                    "error_count": 3,
                    "error_latency": 6.5,
                    "status_codes": {
                        "200": 47,
                        "500": 1,
                        "404": 2
                    }
                }
            ]'::jsonb
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION insert_sample_data(num_records INTEGER)
RETURNS VOID AS $$
DECLARE
    i INTEGER;
    j INTEGER;
    feature_json JSONB;
    profile_json JSONB;
    drift_values TEXT := '';
BEGIN
    FOR i IN 1..num_records LOOP
        -- Generate feature JSON
        feature_json := '{}'::JSONB;
        FOR j IN 1..10 LOOP
            feature_json := feature_json || jsonb_build_object(
                'col_' || j, jsonb_build_object(
                    'id', 'col_' || j,
                    'center', random(),
                    'one_ucl', random() + 1,
                    'one_lcl', random() - 1,
                    'two_ucl', random() + 2,
                    'two_lcl', random() - 2,
                    'three_ucl', random() + 3,
                    'three_lcl', random() - 3,
                    'timestamp', '2024-06-26T20:43:27.957229'
                )
            );
        END LOOP;

        -- Construct the full profile JSON
        profile_json := jsonb_build_object(
            'features', feature_json,
            'config', jsonb_build_object(
                'sample_size', 25,
                'sample', true,
                'name', 'model-' || i,
                'repository', 'ml-platform-' || i,
                'version', '0.1.0',
                'alert_config', jsonb_build_object(
                    'dispatch_type', 'Console',
                    'schedule', '0 0 0 * * *',
                    'rule', jsonb_build_object(
                            'rule', '8 16 4 8 2 4 1 1',
                            'zones_to_monitor', ARRAY['Zone 1', 'Zone 2', 'Zone 3', 'Zone 4'],
                    ),
                    'features_to_monitor', '[]'::JSONB,
                    'dispatch_kwargs', '{}'::JSONB
                ),
                'feature_map', null,
                'targets', '[]'::JSONB,
                'drift_type': 'SPC',
            ),
            'scouter_version', '0.1.0'
        );

        INSERT INTO scouter.drift_profile (
            created_at, updated_at, name, repository, version, profile, drift_type, active, schedule, next_run, previous_run
        )
        VALUES (
            timezone('utc', now()),
            timezone('utc', now()),
            'model-' || i,
            'ml-platform-' || i,
            '0.1.0',
            profile_json,
            'SPC',
            true,
            '0 0 0 * * *',
            timezone('utc', now() - interval '1 days'),
            timezone('utc', now() - interval '2 days')
        );

        -- Generate drift values for 100 features
        FOR j IN 1..1000 LOOP
            drift_values := '';
            FOR k IN 1..10 LOOP
                drift_values := drift_values || format('(timezone(''utc'', now() + interval ''%s seconds''), ''model-%s'', ''ml-platform-%s'', ''col_%s'', %s, ''0.1.0'')', (j - 1) * 20 + (k - 1), i, i, k, normal_rand(1, 0, 1));
                IF k < 10 THEN
                    drift_values := drift_values || ',';
                END IF;
            END LOOP;

            EXECUTE format('
                INSERT INTO scouter.drift (created_at, name, repository, feature, value, version)
                VALUES %s', drift_values);
        END LOOP;
    END LOOP;

    -- Call the insert_observability_data function
    PERFORM insert_observability_data(num_records);
END;
$$ LANGUAGE plpgsql;

-- Call the function to insert 1 record (change this number as needed)
SELECT insert_sample_data(1);