CREATE TABLE tracker_scores (
    tracker_id integer NOT NULL REFERENCES trackers (id) ON DELETE CASCADE,
    axis rating_axis NOT NULL,
    score double precision NOT NULL,
    effective_weight double precision NOT NULL,
    vote_count integer NOT NULL,
    refreshed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tracker_id, axis)
);

CREATE TABLE pairwise_scores (
    tracker_id integer PRIMARY KEY REFERENCES trackers (id) ON DELETE CASCADE,
    strength double precision NOT NULL,
    vote_count integer NOT NULL,
    refreshed_at timestamptz NOT NULL DEFAULT now()
);

