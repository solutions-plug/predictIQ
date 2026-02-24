INSERT INTO contact_form_submissions (name, email, subject, message, status, submitted_at, resolved_at, metadata)
VALUES
    ('Ruth Clark', 'ruth@example.com', 'Partnership Inquiry', 'Interested in a strategic integration.', 'new', NOW() - INTERVAL '1 day', NULL, '{"channel":"website"}'::jsonb),
    ('Ben Doe', 'ben@example.com', 'Support', 'Need help with account access.', 'resolved', NOW() - INTERVAL '4 days', NOW() - INTERVAL '3 days', '{"priority":"high"}'::jsonb),
    ('Nia Stone', 'nia@example.com', 'Media', 'Requesting a press kit and founder bio.', 'in_progress', NOW() - INTERVAL '2 days', NULL, '{"channel":"email"}'::jsonb);
