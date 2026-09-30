# Frontend

This package contains the Handsoff web frontend.

## Documentation

- [Styling conventions](../docs/styling.md) — CSP-safe styling rules and conventions for frontend contributors.
- [Internationalization (i18n) guide](./I18N_GUIDE.md) — how to add and manage translations in the frontend.
- [Design system](./docs/design-system.md) — design system guidelines and conventions for the frontend.
- [`docs/`](./docs/) — additional frontend documentation.

## Styling conventions (CSP-safe, no inline styles)

### Why: the CSP constraint

Our production Content-Security-Policy does **not** allow inline styles. As a
result, any `style={{ ... }}` prop passed to a component is silently dropped by
the browser — the element renders with no styling at all, and the bug is easy to
miss in development where the policy may be relaxed.

This was a real production incident: commits `5bd5e51` ("move AppShell chrome
off inline styles, CSP was dropping them") and `e80a15b` ("migrate every
remaining inline style prop to CSS classes") fixed it by removing inline styles
entirely. The constraint is documented here so the same bug is not reintroduced.

### The rule

**Do not use the `style` prop for anything that affects rendering.**

```jsx
// ❌ Wrong — stripped by CSP, element renders unstyled
<div style={{ marginTop: 8, color: 'red' }}>…</div>

// ✅ Correct — use a CSS class
<div className="stack-sm text-danger">…</div>
```

Use CSS classes (or CSS modules) for all styling. Dynamic values that would
normally be computed inline should be expressed as a class variant, a CSS custom
property set through a class, or a data attribute selected in CSS — never as an
inline `style` prop.

### Enforcement

There is currently **no lint rule or CI check** enforcing this, so it relies on
review. We should add one: an ESLint rule such as
[`react/forbid-dom-props`](https://github.com/jsx-eslint/eslint-plugin-react/blob/master/docs/rules/forbid-dom-props.md)
configured to forbid the `style` prop would catch regressions at lint time:

```json
{
  "rules": {
    "react/forbid-dom-props": ["error", { "forbid": ["style"] }]
  }
}
```

Until that rule (or an equivalent CI check) lands, treat any new `style` prop as
a review blocker.

## Further reading

- [Internationalization (i18n) guide](./I18N_GUIDE.md) — how to add and manage
  translations in the frontend.
- [`docs/`](./docs/) — additional frontend documentation.