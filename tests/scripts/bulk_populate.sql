-- Create a function to insert records
CREATE OR REPLACE FUNCTION insert_sample_data(num_records INTEGER)
RETURNS VOID AS $$
DECLARE
    i INTEGER;
BEGIN
    FOR i IN 1..num_records LOOP
        INSERT INTO scouter.drift_profile (created_at, updated_at, name, repository, version, profile, active, schedule, next_run, previous_run)
        VALUES
        (
            timezone('utc', now()),
            timezone('utc', now()),
            'test_app' || i,
            'opsml',
            '0.1.0',
            '{
            "features": {
                "col_2": {
                "id": "col_2",
                "center": -4.090610111507429,
                "one_ucl": -2.146102177058493,
                "one_lcl": -6.035118045956365,
                "two_ucl": -0.20159424260955694,
                "two_lcl": -7.9796259804053005,
                "three_ucl": 1.7429136918393793,
                "three_lcl": -9.924133914854238,
                "timestamp": "2024-06-26T20:43:27.957229"
                },
                "col_1": {
                "id": "col_1",
                "center": -3.997113080300062,
                "one_ucl": -1.9742384896265417,
                "one_lcl": -6.019987670973582,
                "two_ucl": 0.048636101046978464,
                "two_lcl": -8.042862261647102,
                "three_ucl": 2.071510691720498,
                "three_lcl": -10.065736852320622,
                "timestamp": "2024-06-26T20:43:27.957229"
                },
                "col_3": {
                "id": "col_3",
                "center": -3.937652409303277,
                "one_ucl": -2.0275656995100224,
                "one_lcl": -5.8477391190965315,
                "two_ucl": -0.1174789897167674,
                "two_lcl": -7.757825828889787,
                "three_ucl": 1.7926077200764872,
                "three_lcl": -9.66791253868304,
                "timestamp": "2024-06-26T20:43:27.957230"
                }
            },
            "config": {
                "sample_size": 25,
                "sample": true,
                "name": "test_app",
                "repository": "opsml",
                "version": "0.1.0",
                "alert_config": {
                    "alert_dispatch_type": "Console",
                    "schedule": "0 0 0 * * *",
                    "alert_rule": {
                        "process": {
                            "rule": "16 16 4 8 2 4 1 1"
                        },
                        "percentage": null
                    }
                }
            }
            }',
            true,
            '0 0 0 * * *',
            timezone('utc', now() - interval '1 days'),
            timezone('utc', now() - interval '2 days')
        );

        FOR j IN 1..1000 LOOP
          INSERT INTO scouter.drift (created_at, name, repository, feature, value, version)
            VALUES
            (timezone('utc', now()), 'test_app' || i, 'opsml', 'col_1', random() - 4, '0.1.0'),
            (timezone('utc', now()), 'test_app' || i, 'opsml', 'col_2', random() - 4, '0.1.0'),
            (timezone('utc', now()), 'test_app' || i, 'opsml', 'col_3', random() + 2, '0.1.0');
        END LOOP;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Call the function to insert 1000 records
SELECT insert_sample_data(1000);