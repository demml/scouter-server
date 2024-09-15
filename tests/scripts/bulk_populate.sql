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
        FOR j IN 1..100 LOOP
            feature_json := feature_json || jsonb_build_object(
                'col_' || j, jsonb_build_object(
                    'id', 'col_' || j,
                    'center', random() * 10 - 5,
                    'one_ucl', random() * 10 - 5,
                    'one_lcl', random() * 10 - 5,
                    'two_ucl', random() * 10 - 5,
                    'two_lcl', random() * 10 - 5,
                    'three_ucl', random() * 10 - 5,
                    'three_lcl', random() * 10 - 5,
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
                'name', 'test_app',
                'repository', 'opsml',
                'version', '0.1.0',
                'alert_config', jsonb_build_object(
                    'alert_dispatch_type', 'Console',
                    'schedule', '0 0 0 * * *',
                    'alert_rule', jsonb_build_object(
                        'process', jsonb_build_object(
                            'rule', '8 16 4 8 2 4 1 1',
                            'zones_to_monitor', ARRAY['Zone 1', 'Zone 2', 'Zone 3', 'Zone 4']
                        ),
                        'percentage', null
                    ),
                    'features_to_monitor', '[]'::JSONB,
                    'alert_kwargs', '{}'::JSONB
                ),
                'feature_map', null,
                'targets', '[]'::JSONB
            ),
            'scouter_version', '0.1.0'
        );

        INSERT INTO scouter.drift_profile (
            created_at, updated_at, name, repository, version, profile, active, schedule, next_run, previous_run
        )
        VALUES (
            timezone('utc', now()),
            timezone('utc', now()),
            'test_app' || i,
            'opsml',
            '0.1.0',
            profile_json,
            true,
            '0 0 0 * * *',
            timezone('utc', now() - interval '1 days'),
            timezone('utc', now() - interval '2 days')
        );

        -- Generate drift values for 100 features
        FOR j IN 1..1000 LOOP
            drift_values := '';
            FOR k IN 1..100 LOOP
                drift_values := drift_values || format('(timezone(''utc'', now()), ''test_app%s'', ''opsml'', ''col_%s'', %s, ''0.1.0'')', i, k, random() * 10 - 5);
                IF k < 100 THEN
                    drift_values := drift_values || ',';
                END IF;
            END LOOP;

            EXECUTE format('
                INSERT INTO scouter.drift (created_at, name, repository, feature, value, version)
                VALUES %s', drift_values);
        END LOOP;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Call the function to insert 1 record (change this number as needed)
SELECT insert_sample_data(1);