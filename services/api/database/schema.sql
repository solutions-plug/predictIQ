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
    unsubscribed_count integer     NOT NULL DEFAULT 0,
    created_at        timestamptz  NOT NULL DEFAULT now(),
    updated_at        timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT email_analytics_pkey PRIMARY KEY (id),
    CONSTRAINT email_analytics_template_name_variant_name_date_key UNIQUE (template_name, variant_name, date)
);

-- migration 016: markets table with consistency constraints
-- Invariants mirror contracts/predict-iq/INVARIANTS.md for stored market state.
CREATE TABLE IF NOT EXISTS public.markets (
    id                  uuid         NOT NULL DEFAULT gen_random_uuid(),
    on_chain_market_id  varchar(120) NOT NULL,
    question            text         NOT NULL,
    outcome_count       integer      NOT NULL,
    status              varchar(50)  NOT NULL DEFAULT 'open',
    winning_outcome     integer,
    resolved_at         timestamptz,
    created_at          timestamptz  NOT NULL DEFAULT now(),
    updated_at          timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT markets_pkey PRIMARY KEY (id),
    -- Uniqueness: each on-chain market is persisted at most once.
    CONSTRAINT markets_on_chain_market_id_key UNIQUE (on_chain_market_id),
    -- Outcome count: a market must offer at least two outcomes.
    CONSTRAINT markets_outcome_count_check CHECK (outcome_count >= 2),
    -- Status domain: only the contract's lifecycle states are storable.
    CONSTRAINT markets_status_check CHECK (status IN ('open', 'closed', 'resolved')),
    -- Winning outcome: must reference a valid outcome index when set.
    CONSTRAINT markets_winning_outcome_check CHECK (
        winning_outcome IS NULL
        OR (winning_outcome >= 0 AND winning_outcome < outcome_count)
    ),
    -- Status transitions: only resolved markets carry a winning outcome and
    -- a resolution timestamp; unresolved markets must not.
    CONSTRAINT markets_resolution_consistency_check CHECK (
        (status = 'resolved' AND winning_outcome IS NOT NULL AND resolved_at IS NOT NULL)
        OR (status <> 'resolved' AND winning_outcome IS NULL AND resolved_at IS NULL)
    )
);
