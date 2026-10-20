DELETE FROM pairwise_votes p
USING pairwise_votes q
WHERE p.voter_id = q.voter_id
  AND LEAST(p.winner_id, p.loser_id) = LEAST(q.winner_id, q.loser_id)
  AND GREATEST(p.winner_id, p.loser_id) = GREATEST(q.winner_id, q.loser_id)
  AND (p.created_at, p.id) < (q.created_at, q.id);

DROP INDEX IF EXISTS pairwise_votes_voter_id_idx;

CREATE UNIQUE INDEX pairwise_votes_voter_pair_idx ON pairwise_votes (
    voter_id,
    LEAST(winner_id, loser_id),
    GREATEST(winner_id, loser_id)
);
