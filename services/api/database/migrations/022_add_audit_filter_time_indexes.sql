-- Support action and resource-type filters combined with timestamp ranges/order.
CREATE INDEX IF NOT EXISTS idx_audit_log_action_time
    ON audit_log (action, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_log_resource_type_time
    ON audit_log (resource_type, timestamp DESC);