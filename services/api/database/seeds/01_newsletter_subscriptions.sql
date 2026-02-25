INSERT INTO newsletter_subscriptions (email, status, source, subscribed_at, confirmed_at)
VALUES
    ('alice@example.com', 'confirmed', 'homepage', NOW() - INTERVAL '7 days', NOW() - INTERVAL '6 days'),
    ('brenda@example.com', 'pending', 'blog', NOW() - INTERVAL '2 days', NULL),
    ('charles@example.com', 'unsubscribed', 'referral', NOW() - INTERVAL '20 days', NOW() - INTERVAL '19 days')
ON CONFLICT (email) DO NOTHING;
