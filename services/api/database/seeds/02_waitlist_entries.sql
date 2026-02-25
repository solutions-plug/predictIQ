INSERT INTO waitlist_entries (email, status, source, priority_score, joined_at, converted_at)
VALUES
    ('earlybird@example.com', 'pending', 'landing-page', 90, NOW() - INTERVAL '3 days', NULL),
    ('partnerlead@example.com', 'invited', 'partnership', 75, NOW() - INTERVAL '12 days', NULL),
    ('converted@example.com', 'converted', 'ads', 55, NOW() - INTERVAL '30 days', NOW() - INTERVAL '5 days')
ON CONFLICT (email) DO NOTHING;
