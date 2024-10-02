-- Add migration script here
CREATE TABLE IF NOT exists scouter.drift_alerts (
  created_at timestamp not null default (timezone('utc', now())),
  name varchar(256) not null,
  repository varchar(256) not null,
  version varchar(256) not null,
  feature varchar(256) not null,
  alert jsonb not null default '{}',
  status varchar(32) not null default 'active',
  id integer generated by default as identity,
  UNIQUE (created_at,name,repository,version)
)
PARTITION BY RANGE (created_at);

CREATE INDEX ON scouter.drift_alerts (name, repository, version, created_at);

SELECT scouter.create_parent(
    'scouter.drift_alerts', 
    'created_at',
    '1 day'
);

UPDATE scouter.part_config SET retention = '7 days' WHERE parent_table = 'scouter.alerts';

-- Add scouter_version column to drift_profile, this can be used for backward compatibility checks down the road
ALTER TABLE scouter.drift_profile
add column scouter_version varchar(256) not null default '0.1.0';
