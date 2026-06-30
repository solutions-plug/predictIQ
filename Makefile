.PHONY: test-integration test-integration-down test-unit help

COMPOSE_TEST = docker compose -f docker-compose.test.yml

TEST_DATABASE_URL = postgres://predictiq_test:predictiq_test@localhost:5433/predictiq_test
TEST_REDIS_URL    = redis://localhost:6380
STELLAR_RPC_URL   = http://localhost:8080

##@ Testing

help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-25s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

test-integration: ## Start backing services, run API integration tests, then tear down
	@echo "==> Starting test services..."
	$(COMPOSE_TEST) up -d --wait
	@echo "==> Running integration tests..."
	@cd services/api && \
		TEST_DATABASE_URL=$(TEST_DATABASE_URL) \
		TEST_REDIS_URL=$(TEST_REDIS_URL) \
		STELLAR_RPC_URL=$(STELLAR_RPC_URL) \
		cargo test --test '*' -- --test-threads=1; \
	STATUS=$$?; \
	echo "==> Tearing down test services..."; \
	cd ../.. && $(COMPOSE_TEST) down -v; \
	exit $$STATUS

test-integration-down: ## Tear down test services (cleanup after a failed run)
	$(COMPOSE_TEST) down -v

test-unit: ## Run unit tests (no external services needed)
	cargo test --lib --workspace
