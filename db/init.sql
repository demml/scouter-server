CREATE SCHEMA if not exists monitoring;
CREATE EXTENSION if not exists pg_partman SCHEMA monitoring;

CREATE ROLE partman_user WITH LOGIN;
GRANT ALL ON SCHEMA monitoring TO partman_user;
GRANT ALL ON ALL TABLES IN SCHEMA monitoring TO partman_user;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA monitoring TO partman_user;
GRANT EXECUTE ON ALL PROCEDURES IN SCHEMA monitoring TO partman_user;
GRANT ALL ON SCHEMA monitoring TO partman_user;
GRANT TEMPORARY ON DATABASE monitor to partman_user;

CREATE TABLE monitoring.scouter (
  created_at timestamp not null default (timezone('utc', now())),
  service_name varchar(256),
  feature varchar(256),
  value double precision
)
PARTITION BY RANGE (created_at);


CREATE INDEX ON monitoring.scouter (service_name, created_at);

SELECT monitoring.create_parent(
    'monitoring.scouter', 
    'created_at',
    '1 day'
);

UPDATE monitoring.part_config SET retention = '1 days' WHERE parent_table = 'monitoring.scouter';