CREATE EXTENSION IF NOT EXISTS tablefunc;

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
END;
$$ LANGUAGE plpgsql;

-- Call the function to insert 1 record (change this number as needed)
SELECT insert_sample_data(1);