-- Add migration script here
CREATE TABLE IF NOT exists scouter.alerts (
  created_at timestamp not null default (timezone('utc', now())),
  name varchar(256),
  repository varchar(256),
  version varchar(256),
  alert jsonb,
  UNIQUE (created_at,name,repository,version)
)
PARTITION BY RANGE (created_at);

CREATE INDEX ON scouter.alerts (name, repository, version, created_at);

SELECT scouter.create_parent(
    'scouter.alerts', 
    'created_at',
    '1 day'
);

UPDATE scouter.part_config SET retention = '7 days' WHERE parent_table = 'scouter.alerts';