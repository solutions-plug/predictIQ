INSERT INTO audit_logs (action, entity_type, entity_id, actor_email, reason, changes, newsletter_subscription_id)
SELECT
    'newsletter_status_updated',
    'newsletter_subscriptions',
    ns.id,
    'system@predictiq.local',
    'Auto-confirmation after email verification',
    '{"from":"pending","to":"confirmed"}'::jsonb,
    ns.id
FROM newsletter_subscriptions ns
WHERE ns.email = 'alice@example.com'
LIMIT 1;

INSERT INTO audit_logs (action, entity_type, entity_id, actor_email, reason, changes, content_id)
SELECT
    'content_published',
    'content_management',
    c.id,
    'editor@example.com',
    'Editorial approval completed',
    '{"from":"draft","to":"published"}'::jsonb,
    c.id
FROM content_management c
WHERE c.slug = 'weekly-market-outlook'
LIMIT 1;
