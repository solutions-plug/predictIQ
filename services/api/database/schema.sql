-- Expected database schema snapshot.
-- Generated from: services/api/database/migrations/ (migrations 000–016)
-- Update workflow: run `pg_dump --schema-only` after applying all migrations
-- on a fresh database, then commit the result here.
--
-- The schema-drift CI job (schema-drift-check in .github/workflows/test.yml)
-- applies all migrations to a fresh PostgreSQL instance, dumps the schema,
-- and diffs it against this file. Any discrepancy fails the build.

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

-- Extension: pgcrypto (migration 001)
CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;

-- ── Tables ────────────────────────────────────────────────────────────────────

-- migration 000
CREATE TABLE IF NOT EXISTS public.schema_migrations (
    version     text        NOT NULL,
    name        text        NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now(),
    checksum    text        NOT NULL,
    CONSTRAINT schema_migrations_pkey PRIMARY KEY (version)
);

-- migration 002
CREATE TABLE IF NOT EXISTS public.newsletter_subscribers (
    id                  uuid         NOT NULL DEFAULT gen_random_uuid(),
    email               varchar(255) NOT NULL,
    source              varchar(100) NOT NULL DEFAULT 'direct',
    confirmed           boolean      NOT NULL DEFAULT false,
    confirmation_token  varchar(255),
    created_at          timestamptz  NOT NULL DEFAULT now(),
    confirmed_at        timestamptz,
    unsubscribed_at     timestamptz,
    updated_at          timestamptz  NOT NULL DEFAULT now(),
    deleted_at          timestamptz,
    CONSTRAINT newsletter_subscribers_pkey    PRIMARY KEY (id),
    CONSTRAINT newsletter_subscribers_email_key UNIQUE (email)
);

-- migration 003
CREATE TABLE IF NOT EXISTS public.contact_form_submissions (
    id           uuid         NOT NULL DEFAULT gen_random_uuid(),
    name         varchar(120) NOT NULL,
    email        varchar(255) NOT NULL,
    subject      varchar(200) NOT NULL,
    message      text         NOT NULL,
    status       varchar(50)  NOT NULL DEFAULT 'new',
    submitted_at timestamptz  NOT NULL DEFAULT now(),
    resolved_at  timestamptz,
    metadata     jsonb        NOT NULL DEFAULT '{}'::jsonb,
    created_at   timestamptz  NOT NULL DEFAULT now(),
    updated_at   timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT contact_form_submissions_pkey PRIMARY KEY (id)
);

-- migration 004
CREATE TABLE IF NOT EXISTS public.waitlist_entries (
    id             uuid         NOT NULL DEFAULT gen_random_uuid(),
    email          varchar(255) NOT NULL,
    status         varchar(50)  NOT NULL DEFAULT 'pending',
    source         varchar(100),
    priority_score integer      NOT NULL DEFAULT 0,
    joined_at      timestamptz  NOT NULL DEFAULT now(),
    converted_at   timestamptz,
    created_at     timestamptz  NOT NULL DEFAULT now(),
    updated_at     timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT waitlist_entries_pkey  PRIMARY KEY (id),
    CONSTRAINT waitlist_entries_email_key UNIQUE (email)
);

-- migration 005
CREATE TABLE IF NOT EXISTS public.content_management (
    id           uuid         NOT NULL DEFAULT gen_random_uuid(),
    slug         varchar(180) NOT NULL,
    title        varchar(220) NOT NULL,
    body         text         NOT NULL,
    excerpt      text,
    status       varchar(50)  NOT NULL DEFAULT 'draft',
    author_email varchar(255) NOT NULL,
    version      integer      NOT NULL DEFAULT 1,
    metadata     jsonb        NOT NULL DEFAULT '{}'::jsonb,
    published_at timestamptz,
    created_at   timestamptz  NOT NULL DEFAULT now(),
    updated_at   timestamptz  NOT NULL DEFAULT now(),
    deleted_at   timestamptz,
    CONSTRAINT content_management_pkey     PRIMARY KEY (id),
    CONSTRAINT content_management_slug_key UNIQUE (slug)
);

-- migration 006
CREATE TABLE IF NOT EXISTS public.analytics_events (
    id             uuid         NOT NULL DEFAULT gen_random_uuid(),
    event_name     varchar(120) NOT NULL,
    event_category varchar(80),
    user_id        uuid,
    session_id     varchar(120),
    page_url       text,
    referrer       text,
    properties     jsonb        NOT NULL DEFAULT '{}'::jsonb,
    ip_address     inet,
    user_agent     text,
    occurred_at    timestamptz  NOT NULL DEFAULT now(),
    created_at     timestamptz  NOT NULL DEFAULT now(),
    content_id     uuid,
    CONSTRAINT analytics_events_pkey     PRIMARY KEY (id),
    CONSTRAINT fk_analytics_content      FOREIGN KEY (content_id)
        REFERENCES public.content_management(id) ON DELETE SET NULL
);

-- migration 007
CREATE TABLE IF NOT EXISTS public.audit_logs (
    id                       uuid         NOT NULL DEFAULT gen_random_uuid(),
    action                   varchar(120) NOT NULL,
    entity_type              varchar(80)  NOT NULL,
    entity_id                uuid,
    actor_email              varchar(255),
    actor_ip                 inet,
    reason                   text,
    changes                  jsonb        NOT NULL DEFAULT '{}'::jsonb,
    created_at               timestamptz  NOT NULL DEFAULT now(),
    deleted_at               timestamptz,
    newsletter_subscription_id uuid,
    contact_submission_id    uuid,
    waitlist_entry_id        uuid,
    content_id               uuid,
    CONSTRAINT audit_logs_pkey PRIMARY KEY (id),
    CONSTRAINT fk_audit_newsletter FOREIGN KEY (newsletter_subscription_id)
        REFERENCES public.newsletter_subscribers(id) ON DELETE SET NULL,
    CONSTRAINT fk_audit_contact    FOREIGN KEY (contact_submission_id)
        REFERENCES public.contact_form_submissions(id) ON DELETE SET NULL,
    CONSTRAINT fk_audit_waitlist   FOREIGN KEY (waitlist_entry_id)
        REFERENCES public.waitlist_entries(id) ON DELETE SET NULL,
    CONSTRAINT fk_audit_content    FOREIGN KEY (content_id)
        REFERENCES public.content_management(id) ON DELETE SET NULL
);

-- migration 008: email tracking tables
CREATE TABLE IF NOT EXISTS public.email_jobs (
    id              uuid         NOT NULL DEFAULT gen_random_uuid(),
    job_type        varchar(50)  NOT NULL,
    recipient_email varchar(255) NOT NULL,
    template_name   varchar(100) NOT NULL,
    template_data   jsonb        NOT NULL DEFAULT '{}'::jsonb,
    status          varchar(50)  NOT NULL DEFAULT 'pending',
    priority        integer      NOT NULL DEFAULT 0,
    attempts        integer      NOT NULL DEFAULT 0,
    max_attempts    integer      NOT NULL DEFAULT 3,
    scheduled_at    timestamptz  NOT NULL DEFAULT now(),
    started_at      timestamptz,
    completed_at    timestamptz,
    failed_at       timestamptz,
    error_message   text,
    created_at      timestamptz  NOT NULL DEFAULT now(),
    updated_at      timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_jobs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.email_events (
    id              uuid         NOT NULL DEFAULT gen_random_uuid(),
    email_job_id    uuid,
    message_id      varchar(255),
    event_type      varchar(50)  NOT NULL,
    recipient_email varchar(255) NOT NULL,
    timestamp       timestamptz  NOT NULL DEFAULT now(),
    metadata        jsonb        NOT NULL DEFAULT '{}'::jsonb,
    created_at      timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_events_pkey PRIMARY KEY (id),
    CONSTRAINT email_events_email_job_id_fkey FOREIGN KEY (email_job_id)
        REFERENCES public.email_jobs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS public.email_suppressions (
    id               uuid         NOT NULL DEFAULT gen_random_uuid(),
    email            varchar(255) NOT NULL,
    suppression_type varchar(50)  NOT NULL,
    reason           text,
    bounce_type      varchar(50),
    created_at       timestamptz  NOT NULL DEFAULT now(),
    updated_at       timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_suppressions_pkey      PRIMARY KEY (id),
    CONSTRAINT email_suppressions_email_key UNIQUE (email)
);

CREATE TABLE IF NOT EXISTS public.email_template_variants (
    id            uuid         NOT NULL DEFAULT gen_random_uuid(),
    template_name varchar(100) NOT NULL,
    variant_name  varchar(50)  NOT NULL,
    subject_line  text         NOT NULL,
    html_content  text         NOT NULL,
    text_content  text,
    is_active     boolean      NOT NULL DEFAULT true,
    weight        integer      NOT NULL DEFAULT 100,
    created_at    timestamptz  NOT NULL DEFAULT now(),
    updated_at    timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_template_variants_pkey                        PRIMARY KEY (id),
    CONSTRAINT email_template_variants_template_name_variant_name_key UNIQUE (template_name, variant_name)
);

CREATE TABLE IF NOT EXISTS public.email_analytics (
    id                uuid         NOT NULL DEFAULT gen_random_uuid(),
    template_name     varchar(100) NOT NULL,
    variant_name      varchar(50),
    date              date         NOT NULL,
    sent_count        integer      NOT NULL DEFAULT 0,
    delivered_count   integer      NOT NULL DEFAULT 0,
    opened_count      integer      NOT NULL DEFAULT 0,
    clicked_count     integer      NOT NULL DEFAULT 0,
    bounced_count     integer      NOT NULL DEFAULT 0,
    complained_count  integer      NOT NULL DEFAULT 0,
    unsubscribed_count integer     NOT NULL DEFAULT 0,
    created_at        timestamptz  NOT NULL DEFAULT now(),
    updated_at        timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_analytics_pkey                             PRIMARY KEY (id),
    CONSTRAINT email_analytics_template_name_variant_name_date_key UNIQUE (template_name, variant_name, date)
);

-- migration 010 (audit_log)
CREATE TABLE IF NOT EXISTS public.audit_log (
    id            bigint       NOT NULL GENERATED ALWAYS AS IDENTITY,
    timestamp     timestamptz  NOT NULL DEFAULT now(),
    actor         varchar(255) NOT NULL,
    actor_ip      inet,
    action        varchar(100) NOT NULL,
    resource_type varchar(50)  NOT NULL,
    resource_id   varchar(255),
    details       jsonb,
    status        varchar(20)  NOT NULL DEFAULT 'success',
    error_message text,
    request_id    uuid,
    user_agent    text,
    created_at    timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT audit_log_pkey PRIMARY KEY (id)
);

-- migration 011
CREATE TABLE IF NOT EXISTS public.markets (
    id            bigint           NOT NULL GENERATED ALWAYS AS IDENTITY,
    title         text             NOT NULL,
    status        text             NOT NULL DEFAULT 'active',
    outcome_index integer,
    total_volume  double precision NOT NULL DEFAULT 0,
    ends_at       timestamptz      NOT NULL,
    created_at    timestamptz      NOT NULL DEFAULT now(),
    resolved_at   timestamptz,
    CONSTRAINT markets_pkey          PRIMARY KEY (id),
    CONSTRAINT markets_status_check  CHECK (status IN ('active', 'resolved', 'cancelled'))
);

-- ── Indexes ───────────────────────────────────────────────────────────────────

-- newsletter_subscribers (migrations 002, 009, 010, 015)
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_email
    ON public.newsletter_subscribers (email);
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_confirmation_token
    ON public.newsletter_subscribers (confirmation_token)
    WHERE confirmation_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_created_at
    ON public.newsletter_subscribers (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_status
    ON public.newsletter_subscribers (confirmed, unsubscribed_at)
    WHERE unsubscribed_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_email_status
    ON public.newsletter_subscribers (email, confirmed)
    WHERE unsubscribed_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_deleted_at
    ON public.newsletter_subscribers (deleted_at)
    WHERE deleted_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_active
    ON public.newsletter_subscribers (email)
    WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_cleanup
    ON public.newsletter_subscribers (created_at ASC)
    WHERE confirmed = false;

-- contact_form_submissions (migration 003)
CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_email
    ON public.contact_form_submissions (email);
CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_status
    ON public.contact_form_submissions (status);
CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_created_at
    ON public.contact_form_submissions (created_at DESC);

-- waitlist_entries (migration 004)
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_email
    ON public.waitlist_entries (email);
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_status
    ON public.waitlist_entries (status);
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_created_at
    ON public.waitlist_entries (created_at DESC);

-- content_management (migration 005)
CREATE INDEX IF NOT EXISTS idx_content_management_slug
    ON public.content_management (slug);
CREATE INDEX IF NOT EXISTS idx_content_management_status
    ON public.content_management (status);
CREATE INDEX IF NOT EXISTS idx_content_management_created_at
    ON public.content_management (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_content_management_deleted_at
    ON public.content_management (deleted_at);

-- analytics_events (migration 006)
CREATE INDEX IF NOT EXISTS idx_analytics_events_event_name
    ON public.analytics_events (event_name);
CREATE INDEX IF NOT EXISTS idx_analytics_events_occurred_at
    ON public.analytics_events (occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_analytics_events_session_id
    ON public.analytics_events (session_id);
CREATE INDEX IF NOT EXISTS idx_analytics_events_created_at
    ON public.analytics_events (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_analytics_events_properties_gin
    ON public.analytics_events USING gin (properties);

-- audit_logs (migration 007)
CREATE INDEX IF NOT EXISTS idx_audit_logs_action
    ON public.audit_logs (action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_type
    ON public.audit_logs (entity_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_id
    ON public.audit_logs (entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at
    ON public.audit_logs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_deleted_at
    ON public.audit_logs (deleted_at);

-- email_jobs (migrations 008, 013)
CREATE INDEX IF NOT EXISTS idx_email_jobs_status
    ON public.email_jobs (status);
CREATE INDEX IF NOT EXISTS idx_email_jobs_scheduled_at
    ON public.email_jobs (scheduled_at);
CREATE INDEX IF NOT EXISTS idx_email_jobs_recipient
    ON public.email_jobs (recipient_email);
CREATE INDEX IF NOT EXISTS idx_email_jobs_type
    ON public.email_jobs (job_type);
CREATE INDEX IF NOT EXISTS idx_email_jobs_status_priority_scheduled
    ON public.email_jobs (status, priority DESC, scheduled_at ASC);

-- email_events (migrations 008, 014)
CREATE INDEX IF NOT EXISTS idx_email_events_job_id
    ON public.email_events (email_job_id);
CREATE INDEX IF NOT EXISTS idx_email_events_message_id
    ON public.email_events (message_id);
CREATE INDEX IF NOT EXISTS idx_email_events_type
    ON public.email_events (event_type);
CREATE INDEX IF NOT EXISTS idx_email_events_recipient
    ON public.email_events (recipient_email);
CREATE INDEX IF NOT EXISTS idx_email_events_timestamp
    ON public.email_events (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_email_events_job_timestamp
    ON public.email_events (email_job_id, timestamp DESC);

-- email_suppressions (migration 008)
CREATE INDEX IF NOT EXISTS idx_email_suppressions_email
    ON public.email_suppressions (email);
CREATE INDEX IF NOT EXISTS idx_email_suppressions_type
    ON public.email_suppressions (suppression_type);

-- email_template_variants (migration 008)
CREATE INDEX IF NOT EXISTS idx_email_template_variants_name
    ON public.email_template_variants (template_name);
CREATE INDEX IF NOT EXISTS idx_email_template_variants_active
    ON public.email_template_variants (is_active);

-- email_analytics (migration 008)
CREATE INDEX IF NOT EXISTS idx_email_analytics_template
    ON public.email_analytics (template_name);
CREATE INDEX IF NOT EXISTS idx_email_analytics_date
    ON public.email_analytics (date DESC);

-- audit_log (migration 010)
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp
    ON public.audit_log (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor
    ON public.audit_log (actor);
CREATE INDEX IF NOT EXISTS idx_audit_log_action
    ON public.audit_log (action);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource
    ON public.audit_log (resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_request_id
    ON public.audit_log (request_id);

-- markets (migrations 011, 012, 016)
CREATE INDEX IF NOT EXISTS markets_status_idx
    ON public.markets (status);
CREATE INDEX IF NOT EXISTS idx_markets_status_volume_ends_at
    ON public.markets (status, total_volume DESC, ends_at ASC);
CREATE INDEX IF NOT EXISTS idx_markets_active_ends_at
    ON public.markets (ends_at ASC)
    WHERE status = 'active';

-- ── Functions & triggers (migration 010) ─────────────────────────────────────

CREATE OR REPLACE FUNCTION public.prevent_audit_log_modification()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Audit log is append-only. Modifications are not allowed.';
END;
$$;

CREATE TRIGGER prevent_audit_log_update
    BEFORE UPDATE ON public.audit_log
    FOR EACH ROW EXECUTE FUNCTION public.prevent_audit_log_modification();

CREATE TRIGGER prevent_audit_log_delete
    BEFORE DELETE ON public.audit_log
    FOR EACH ROW EXECUTE FUNCTION public.prevent_audit_log_modification();

-- ── Views (migration 010) ─────────────────────────────────────────────────────

CREATE OR REPLACE VIEW public.recent_admin_actions AS
SELECT
    id,
    timestamp,
    actor,
    action,
    resource_type,
    resource_id,
    status,
    error_message
FROM public.audit_log
WHERE timestamp > (now() - '30 days'::interval)
ORDER BY timestamp DESC;
