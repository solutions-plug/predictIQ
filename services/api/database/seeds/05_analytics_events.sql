INSERT INTO analytics_events (event_name, event_category, session_id, page_url, referrer, properties, occurred_at, content_id)
SELECT
    'newsletter_signup',
    'conversion',
    'sess-001',
    '/newsletter',
    'https://google.com',
    '{"source":"homepage"}'::jsonb,
    NOW() - INTERVAL '1 day',
    c.id
FROM content_management c
WHERE c.slug = 'welcome-to-predictiq'
LIMIT 1;

INSERT INTO analytics_events (event_name, event_category, session_id, page_url, referrer, properties, occurred_at)
VALUES
    ('page_view', 'engagement', 'sess-002', '/blog/weekly-market-outlook', 'https://x.com', '{"device":"mobile"}'::jsonb, NOW() - INTERVAL '12 hours'),
    ('waitlist_join', 'conversion', 'sess-003', '/waitlist', 'https://linkedin.com', '{"campaign":"spring"}'::jsonb, NOW() - INTERVAL '6 hours');
