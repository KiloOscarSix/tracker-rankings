CREATE TYPE membership_status AS ENUM (
    'current',
    'former'
);

CREATE TYPE rating_axis AS ENUM (
    'overall',
    'content',
    'community',
    'retention',
    'longevity'
);

CREATE TABLE memberships (
    voter_id bigint NOT NULL REFERENCES voters (id) ON DELETE CASCADE,
    tracker_id integer NOT NULL REFERENCES trackers (id) ON DELETE RESTRICT,
    status membership_status NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (voter_id, tracker_id)
);

CREATE INDEX memberships_tracker_id_idx ON memberships (tracker_id);

CREATE TABLE ratings (
    voter_id bigint NOT NULL,
    tracker_id integer NOT NULL,
    axis rating_axis NOT NULL,
    score smallint NOT NULL CHECK (score BETWEEN 1 AND 5),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (voter_id, tracker_id, axis),
    FOREIGN KEY (voter_id, tracker_id) REFERENCES memberships (voter_id, tracker_id) ON DELETE CASCADE
);

CREATE INDEX ratings_tracker_id_axis_idx ON ratings (tracker_id, axis);

CREATE TABLE pairwise_votes (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    voter_id bigint NOT NULL REFERENCES voters (id) ON DELETE CASCADE,
    winner_id integer NOT NULL REFERENCES trackers (id) ON DELETE RESTRICT,
    loser_id integer NOT NULL REFERENCES trackers (id) ON DELETE RESTRICT,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (winner_id <> loser_id)
);

CREATE INDEX pairwise_votes_voter_id_idx ON pairwise_votes (voter_id);

