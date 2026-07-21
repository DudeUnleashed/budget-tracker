-- New installs no longer seed built-in cross-ledger tags (Owner's Draw / Business Loan /
-- Loan Repayment) - Categories now gives users enough control to set these up themselves if they
-- want them, so auto-generating them by default is unnecessary clutter. This cleans them up for
-- databases that already have them from before, but only where they're not actually in use - a
-- transaction, recurring item, or budget still referencing one, or another tag still nested under
-- one - since force-deleting an in-use tag would cascade and silently strip that real data.
DELETE FROM tags
WHERE name IN ('Owner''s Draw', 'Business Loan', 'Loan Repayment')
  AND id NOT IN (SELECT tag_id FROM transaction_tags)
  AND id NOT IN (SELECT tag_id FROM recurring)
  AND id NOT IN (SELECT tag_id FROM budgets WHERE tag_id IS NOT NULL)
  AND id NOT IN (SELECT parent_id FROM tags WHERE parent_id IS NOT NULL);
