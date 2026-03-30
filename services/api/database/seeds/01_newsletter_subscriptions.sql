INSERT INTO newsletter_subscribers (email, source, confirmed, confirmation_token, created_at, confirmed_at, unsubscribed_at)
VALUES
    ('alice@example.com',   'homepage', TRUE,  NULL,                                    NOW() - INTERVAL '7 days',  NOW() - INTERVAL '6 days', NULL),
    ('brenda@example.com',  'blog',     FALSE, 'tok_brenda_pending_000000000000000000', NOW() - INTERVAL '2 days',  NULL,                      NULL),
    ('charles@example.com', 'referral', FALSE, NULL,                                    NOW() - INTERVAL '20 days', NOW() - INTERVAL '19 days', NOW() - INTERVAL '1 day')
ON CONFLICT (email) DO NOTHING;
