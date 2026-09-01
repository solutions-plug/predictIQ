# Landing page: Step, FooterColumn, SEO metadata, responsive

Four landing-page issues.

## What changed and why

### #1345 - Step component + real "how it works" flow
- `Step` takes an optional `href` and links the step title to the corresponding feature.
  The **step number is not passed in** - `landing.css` already derives it with a CSS
  `counter()` from the `<ol>` order, so it can't drift from the visual list.
- The four steps now describe the real, shipped flow: connect a Stellar wallet -> browse
  markets -> place a bet -> claim a payout (i18n `howItWorks.*`), each linking to `/markets`
  or `/account/bets`.
- Tests: title links to its feature; plain title without `href`; no hardcoded numbering in
  the DOM; the `LandingPage` data-driven test asserts the `<ol>` and the four step links.

### #1346 - FooterColumn + real footer navigation
- `FooterColumn` gains a `children` slot (for embeds) and an `external` link flag
  (`target="_blank"` + `rel="noopener noreferrer"`).
- The footer columns were rewritten to routes that **exist**: Product (Markets `/markets`,
  Statistics `/statistics`, Create a market `/markets/create`), Resources (Documentation +
  GitHub, both external), and a "stay in the loop" column linking back to the hero signup
  form (a second `<NewsletterSignup>` would duplicate its field ids and break a11y). The
  dead `/docs`, `/github` (internal), `/discord`, `/privacy`, `/terms` links and the Legal
  column were removed - there are no such pages.
- `e2e/landing-links-responsive.spec.ts` asserts every internal footer link resolves
  (`request.get(href)` < 400) - the CI link-check the AC asks for.
- Tests: external vs internal link attributes; `children` render; the data-driven test now
  asserts real routes and the absence of the removed links.

### #1347 - SEO metadata + structured data
- `app/layout.tsx`: site-wide defaults - `metadataBase`, a `title` template, Open Graph
  `siteName`/`url`, Twitter `summary_large_image`.
- `app/page.tsx` (kept server-side, still code-split via `next/dynamic` with `ssr: true` and
  the existing loading fallback): route-level `title` / `description` / canonical / full
  Open Graph + Twitter with an `/og-image.png`, plus an `Organization` + `WebSite` JSON-LD
  `<script type="application/ld+json">` (a build-time constant, server-rendered so
  link-unfurl crawlers see it).
- Locale-specific metadata (#25) is out of scope here - the i18n layer only has an `en`
  bundle today; `SITE_URL` reads `NEXT_PUBLIC_SITE_URL` with a default.
- Tests: `landing-metadata.test.ts` asserts the exported `metadata` shape and the layout
  defaults.

### #1348 - responsive hardening
- `overflow-wrap: anywhere` on hero/feature/step/footer text so a long word in a translated
  locale wraps instead of widening the page.
- Grid tracks changed to `minmax(min(100%, 230px), 1fr)` so a track can shrink below its
  content's min-content size; `.footer-content { max-width: 100% }`.
- A `@media (max-width: 360px)` block forces single-column grids and trims inline padding.
- `e2e/landing-links-responsive.spec.ts` asserts `scrollWidth - clientWidth <= 1` at 320 /
  768 / 1440.

## How to test

```
cd frontend
PUPPETEER_SKIP_DOWNLOAD=true npm ci --legacy-peer-deps --ignore-scripts
./node_modules/.bin/jest src/components/__tests__/Step.test.tsx \
  src/components/__tests__/FooterColumn.test.tsx \
  src/components/__tests__/LandingPage.dataDriven.test.tsx \
  src/app/__tests__/landing-metadata.test.tsx
```

- 37 tests pass across the touched suites; `page.test.tsx` (the dynamic-loading fallback
  test) still passes - the code-split + loading spinner were preserved.
- `tsc --noEmit`: no errors in the touched files over the repo's pre-existing count.
- `e2e/landing-links-responsive.spec.ts` not run here (needs the browser install).
- Pre-existing on `main`, unchanged by this branch (verified against a clean `upstream/main`
  worktree - identical 30 failures): `Statistics.test.tsx`, `LandingPage.keyboard.test.tsx`,
  `LandingPage.accessibility.test.tsx`, `useAsync.test.ts`, `useReferral.test.ts`.

## Breaking changes

None. New props and metadata only. Footer link targets changed (removed dead links).

## Related issues

Closes #1345
Closes #1346
Closes #1347
Closes #1348

## PR Checklist

- [x] Branch is up to date with `main`
- [x] Commit messages follow Conventional Commits
- [x] Tests added or updated for the change
- [x] Documentation updated if behaviour changed (n/a)
- [x] No secrets or credentials committed
