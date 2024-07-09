-- Add migration script here
CREATE SCHEMA if not exists scouter;
CREATE EXTENSION if not exists pg_partman SCHEMA scouter;

CREATE ROLE partman_user WITH LOGIN;
GRANT ALL ON SCHEMA scouter TO partman_user;
GRANT ALL ON ALL TABLES IN SCHEMA scouter TO partman_user;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA scouter TO partman_user;
GRANT EXECUTE ON ALL PROCEDURES IN SCHEMA scouter TO partman_user;
GRANT ALL ON SCHEMA scouter TO partman_user;
GRANT TEMPORARY ON DATABASE monitor to partman_user;

CREATE TABLE scouter.drift (
  created_at timestamp not null default (timezone('utc', now())),
  name varchar(256),
  repository varchar(256),
  feature varchar(256),
  value double precision,
  version varchar(256),
  UNIQUE (created_at,name,repository,feature,value,version)
)
PARTITION BY RANGE (created_at);


CREATE INDEX ON scouter.drift (name, repository, version, created_at);

SELECT scouter.create_parent(
    'scouter.drift', 
    'created_at',
    '1 day'
);

UPDATE scouter.part_config SET retention = '1 days' WHERE parent_table = 'scouter.drift';


-- Create table for service drift configuration
CREATE TABLE scouter.drift_profile (
  created_at timestamp not null default (timezone('utc', now())),
  updated_at timestamp not null default (timezone('utc', now())),
  name varchar(256),
  repository varchar(256),
  version varchar(256),
  profile jsonb,
  active boolean default true,
  cron  varchar(256),
  next_run timestamp,
  PRIMARY KEY (name, repository, version)
);