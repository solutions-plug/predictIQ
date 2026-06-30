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

## Bundle Size

<!-- Run `npm run analyze` in the frontend directory and fill in the table below.
     Baseline: vendor ~220 kB, main ~90 kB, _app ~60 kB (all gzip). -->

| Chunk | Before | After |
|---|---|---|
| vendor.js | | |
| main*.js | | |
| pages/_app*.js | | |

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
- [ ] Bundle size checked (if frontend changes)

## Related Issues

Closes #
