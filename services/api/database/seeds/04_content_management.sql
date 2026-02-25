INSERT INTO content_management (slug, title, body, excerpt, status, author_email, version, metadata, published_at)
VALUES
    ('welcome-to-predictiq', 'Welcome to PredictIQ', 'Initial launch post content.', 'Launch announcement', 'published', 'editor@example.com', 1, '{"tags":["launch","product"]}'::jsonb, NOW() - INTERVAL '10 days'),
    ('weekly-market-outlook', 'Weekly Market Outlook', 'Weekly analysis content body.', 'This week in prediction markets', 'published', 'analyst@example.com', 3, '{"tags":["analysis"]}'::jsonb, NOW() - INTERVAL '2 days'),
    ('roadmap-q2', 'Roadmap Q2', 'Roadmap draft content body.', 'Upcoming platform roadmap', 'draft', 'pm@example.com', 2, '{"internal":true}'::jsonb, NULL)
ON CONFLICT (slug) DO NOTHING;
