#!/bin/bash
set -e

# Email Service Migration Script
# Runs all database migrations for the email service

DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost/predictiq}"

echo "Running email service migrations..."
echo "Database: $DATABASE_URL"

# Run migrations in order
for migration in services/api/database/migrations/*.sql; do
    echo "Running migration: $(basename $migration)"
    psql "$DATABASE_URL" -f "$migration"
done

echo "✅ All migrations completed successfully!"
