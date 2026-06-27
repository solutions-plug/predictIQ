## Description

<!-- What does this PR do? Why is it needed? -->

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor / code cleanup
- [ ] Documentation update
- [ ] CI / tooling change
- [ ] Breaking change

## Testing Done

<!-- Describe how you tested this change. -->

## Checklist

- [ ] Tests pass locally
- [ ] Documentation updated (if applicable)
- [ ] No breaking changes, or breaking changes are documented above
- [ ] If you **added or changed an API endpoint**, regenerated the OpenAPI spec and committed the result:
  ```bash
  cd services/api && cargo run --bin generate-openapi > openapi.yaml
  git add openapi.yaml && git commit -m "chore: regenerate openapi.yaml"
  ```
- [ ] If you **changed system architecture** (new service, database, external dependency, or network boundary), updated [`docs/architecture.md`](../docs/architecture.md)

## Related Issues

Closes #
