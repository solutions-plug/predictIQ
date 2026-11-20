# Frontend

This package contains the Handsoff web frontend.

## Documentation

- [Styling conventions](../docs/styling.md) — CSP-safe styling rules and conventions for frontend contributors.
- [Internationalization (i18n) guide](./I18N_GUIDE.md) — how to add and manage translations in the frontend.
- [Design system](./docs/design-system.md) — design system guidelines and conventions for the frontend.
- [`docs/`](./docs/) — additional frontend documentation.

## On-chain contract writes

`file_dispute`, `cast_vote`, and `claim_winnings` have no API route, so the
pages that need them call the contract directly via
[`src/lib/wallet/soroban.ts`](./src/lib/wallet/soroban.ts): build → wallet-sign
→ submit to Soroban RPC. Configure it with:

| Variable | Purpose |
|---|---|
| `NEXT_PUBLIC_CONTRACT_ID` | The PredictIQ contract id to invoke |
| `NEXT_PUBLIC_SOROBAN_RPC_URL` | Soroban RPC endpoint for the contract's network |
| `NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE` | Network passphrase the wallet signs for (defaults to testnet) |

When `NEXT_PUBLIC_CONTRACT_ID` or `NEXT_PUBLIC_SOROBAN_RPC_URL` is unset, those
pages render a "not configured" message and disable the submit button rather
than calling a nonexistent endpoint. See `.env.example` for examples.


## Admin authentication flow

The admin UI (`/admin/*`) is gated client-side by `AdminAuthGate` in
[`src/app/admin/layout.tsx`](./src/app/admin/layout.tsx). The API key client
lives in [`src/lib/api/admin-client.ts`](./src/lib/api/admin-client.ts).

**Obtaining the key.** There is no username/password login. An operator pastes
an admin API key into the password field shown by the gate.

**Validation.** The gate validates the key by calling
`GET /api/v1/admin/api-keys` (`api.listApiKeys(key)`), sending it as the
`x-api-key` header. There is no separate session endpoint.

**Storage.** On success the key is stored in `sessionStorage` under
`predictiq-admin-key` (`ADMIN_KEY_STORAGE`). It is cleared when the tab closes
or the operator clicks **Logout**. It is never put in `localStorage` or cookies.
Subsequent admin requests read it from `sessionStorage` and attach it as
`x-api-key`.

**Missing key.** The gate renders the key form and no admin page content or
navigation is mounted.

**Invalid key.**

| Situation | Behaviour |
|---|---|
| Submitted key rejected (401/403) | "Invalid API key. Please try again."; key not stored |
| Validation fails for another reason (network, 5xx) | "Failed to validate API key. Please try again."; key not stored |
| Stored key rejected on page load | Key removed from `sessionStorage`; "Session expired. Please log in again." |

**Behavioural specification.**
[`e2e/admin.spec.ts`](./e2e/admin.spec.ts) is the source of truth for how the
API treats admin keys: a missing key or an invalid key must yield `401`. The
suite requires `ADMIN_API_KEY` (or `ADMIN_KEY`) and fails loudly without it.

**Divergence from the intended flow.** Issue #1757 reported that the gate
called a nonexistent session endpoint and unlocked the UI without validating
the key. The current layout validates against `/api/v1/admin/api-keys`, so
that gap is closed in code. Remaining caveats: the gate is a client-side UX
control only (real enforcement is the API's `x-api-key` check on every admin
route), and `e2e/admin.spec.ts` exercises the API directly rather than driving
the gate UI, so the gate's own invalid-key messages are not covered by an e2e
test.

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