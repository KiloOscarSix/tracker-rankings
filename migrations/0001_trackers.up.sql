CREATE TABLE trackers (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug text NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    name text NOT NULL UNIQUE,
    -- False for route endpoints that upstream's trackerInfo does not describe.
    listed boolean NOT NULL,
    abbr text,
    raw_type text,
    description text,
    categories text[] NOT NULL DEFAULT '{}',
    country text,
    founded_year smallint,
    unlock_days integer CHECK (unlock_days >= 0),
    unlock_reqs text,
    source_sha text NOT NULL CHECK (source_sha ~ '^[0-9a-f]{40}$'),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (NOT listed OR (abbr IS NOT NULL AND raw_type IS NOT NULL AND description IS NOT NULL))
);

CREATE TABLE routes (
    source_id integer NOT NULL REFERENCES trackers (id) ON DELETE CASCADE,
    target_id integer NOT NULL REFERENCES trackers (id) ON DELETE CASCADE,
    days integer CHECK (days >= 0),
    reqs text NOT NULL,
    active text NOT NULL,
    updated_label text NOT NULL,
    PRIMARY KEY (source_id, target_id),
    CHECK (source_id <> target_id)
);

CREATE INDEX routes_target_id_idx ON routes (target_id);

