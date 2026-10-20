DROP INDEX IF EXISTS pairwise_votes_voter_pair_idx;

CREATE INDEX pairwise_votes_voter_id_idx ON pairwise_votes (voter_id);
