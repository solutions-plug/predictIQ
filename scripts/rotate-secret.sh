#!/usr/bin/env bash
# rotate-secret.sh — rotate a named AWS Secrets Manager secret and trigger an
# ECS service redeployment so running tasks pick up the new value.
#
# Usage:
#   ./scripts/rotate-secret.sh <secret-name> <new-value> [environment]
#
# Arguments:
#   secret-name   Short name of the secret, e.g. "hmac-key", "sendgrid-api-key",
#                 "api-signing-key", "database-url", "redis-url".
#                 The full Secrets Manager path is constructed as:
#                   predictiq/<environment>/<secret-name>
#   new-value     The new secret value to store.  Pass "-" to read from stdin
#                 (recommended for large values to avoid shell history leakage).
#   environment   Deployment environment: dev | staging | prod  (default: prod)
#
# Prerequisites:
#   - AWS CLI v2 installed and configured with credentials that have:
#       secretsmanager:PutSecretValue on the target secret
#       ecs:UpdateService on the target ECS service
#   - jq installed for JSON parsing
#
# Examples:
#   Rotate the HMAC key in production:
#     ./scripts/rotate-secret.sh hmac-key "$(openssl rand -hex 32)" prod
#
#   Rotate the SendGrid API key, reading value from stdin:
#     echo "SG.newkey..." | ./scripts/rotate-secret.sh sendgrid-api-key - staging
#
# Exit codes:
#   0  Secret rotated and ECS service update triggered successfully.
#   1  Missing required argument or dependency.
#   2  AWS CLI call failed.

set -euo pipefail

# ── colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; YELLOW='\033[1;33m'; GREEN='\033[0;32m'; NC='\033[0m'

info()    { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
die()     { error "$*"; exit 1; }

# ── dependency checks ─────────────────────────────────────────────────────────
command -v aws  >/dev/null 2>&1 || die "aws CLI is not installed or not in PATH"
command -v jq   >/dev/null 2>&1 || die "jq is not installed or not in PATH"

# ── argument parsing ──────────────────────────────────────────────────────────
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <secret-name> <new-value|-> [environment]"
  echo "  secret-name:  hmac-key | sendgrid-api-key | api-signing-key | database-url | redis-url"
  echo "  new-value:    new secret string, or '-' to read from stdin"
  echo "  environment:  dev | staging | prod  (default: prod)"
  exit 1
fi

SECRET_SHORT_NAME="$1"
NEW_VALUE_ARG="$2"
ENVIRONMENT="${3:-prod}"

# Validate environment
if [[ ! "$ENVIRONMENT" =~ ^(dev|staging|prod)$ ]]; then
  die "environment must be one of: dev, staging, prod"
fi

# Validate secret name against the known inventory
VALID_NAMES=("hmac-key" "sendgrid-api-key" "api-signing-key" "database-url" "redis-url")
VALID=false
for name in "${VALID_NAMES[@]}"; do
  [[ "$SECRET_SHORT_NAME" == "$name" ]] && VALID=true && break
done
$VALID || die "Unknown secret name '${SECRET_SHORT_NAME}'. Valid names: ${VALID_NAMES[*]}"

# Read value from stdin if requested
if [[ "$NEW_VALUE_ARG" == "-" ]]; then
  info "Reading new secret value from stdin..."
  NEW_VALUE="$(cat)"
else
  NEW_VALUE="$NEW_VALUE_ARG"
fi

[[ -z "$NEW_VALUE" ]] && die "New secret value must not be empty"

# ── resolve names ─────────────────────────────────────────────────────────────
SECRET_PATH="predictiq/${ENVIRONMENT}/${SECRET_SHORT_NAME}"
ECS_CLUSTER="predictiq-${ENVIRONMENT}"
ECS_SERVICE="predictiq-${ENVIRONMENT}-api"

info "Rotating secret: ${SECRET_PATH}"
info "Environment:     ${ENVIRONMENT}"
info "ECS service:     ${ECS_CLUSTER}/${ECS_SERVICE}"

# ── confirm in production ─────────────────────────────────────────────────────
if [[ "$ENVIRONMENT" == "prod" ]]; then
  warn "You are about to rotate a production secret."
  warn "This will trigger a rolling ECS redeployment."
  read -r -p "Type 'yes' to continue: " CONFIRM
  [[ "$CONFIRM" == "yes" ]] || { info "Aborted."; exit 0; }
fi

# ── update the secret ─────────────────────────────────────────────────────────
info "Updating secret value in AWS Secrets Manager..."
aws secretsmanager put-secret-value \
  --secret-id "${SECRET_PATH}" \
  --secret-string "${NEW_VALUE}" \
  --output json \
  | jq -r '"  Secret version ID: \(.VersionId)"' \
  || die "Failed to update secret '${SECRET_PATH}'"

info "Secret updated successfully."

# ── trigger ECS service update ────────────────────────────────────────────────
# A force-new-deployment causes ECS to start new tasks that will pull the
# updated secret from Secrets Manager on startup.
info "Triggering ECS service redeployment to pick up the new secret..."
aws ecs update-service \
  --cluster "${ECS_CLUSTER}" \
  --service "${ECS_SERVICE}" \
  --force-new-deployment \
  --output json \
  | jq -r '"  ECS deployment ID: \(.service.deployments[0].id)"' \
  || die "Failed to trigger ECS redeployment for ${ECS_CLUSTER}/${ECS_SERVICE}"

info "ECS service update triggered. New tasks will start with the rotated secret."
info "Monitor progress with:"
info "  aws ecs describe-services --cluster ${ECS_CLUSTER} --services ${ECS_SERVICE}"
echo
info "Rotation complete for secret: ${SECRET_PATH}"
