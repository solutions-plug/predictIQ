/**
 * Boundary validation for outgoing request bodies (#1341).
 *
 * Each schema below mirrors a request body defined in the generated OpenAPI
 * types (`components['schemas']` from schema.d.ts, itself generated from
 * services/api/openapi.yaml). Validating a body here — before it crosses the
 * network — turns a confusing server 400 into an immediate, local, testable
 * failure during development.
 *
 * Two rules keep this useful rather than annoying:
 *   1. Objects are validated in *loose* mode (`.loose()`), so a field the
 *      backend accepts as absent, or an extra field a newer backend adds, is
 *      never rejected here. We only assert the shape of what we *do* send.
 *   2. `request-schema-contract.test.ts` asserts, at compile time, that each
 *      schema's inferred type still matches the generated component type, so
 *      a backend contract change that regenerates schema.d.ts breaks the type
 *      check instead of silently drifting.
 */

import { z } from 'zod';
import type { components } from './schema';

type Schemas = components['schemas'];

/** POST /api/v1/newsletter/subscribe */
export const newsletterSubscribeSchema = z
  .object({
    email: z.email('A valid email address is required.'),
    source: z.string().optional(),
  })
  .loose();

/**
 * Shared `{ email }` body — POST/DELETE unsubscribe, GDPR request-token,
 * GDPR delete (openapi `EmailRequest`).
 */
export const emailRequestSchema = z
  .object({
    email: z.email('A valid email address is required.'),
  })
  .loose();

/**
 * GDPR export. openapi types this as `EmailRequest`, but the client also
 * sends an optional `token` once the subscriber has one, so it is allowed
 * here without being required.
 */
export const gdprExportSchema = z
  .object({
    email: z.email('A valid email address is required.'),
    token: z.string().optional(),
  })
  .loose();

/** POST /api/v1/email/test (admin) — openapi `EmailTestRequest`. */
export const emailTestSchema = z
  .object({
    recipient: z.email('A valid recipient email address is required.'),
    template_name: z.string().min(1, 'A template name is required.'),
  })
  .loose();

/**
 * POST /api/v1/blockchain/markets/{market_id}/bets.
 *
 * Not yet in openapi.yaml (see #78), so this shape is asserted directly
 * against the `placeBet` wrapper's own body type rather than a generated
 * component.
 */
export const placeBetSchema = z
  .object({
    wallet: z.string().min(1, 'A connected wallet address is required.'),
    outcome: z.number().int('Outcome must be a whole number.').nonnegative('Outcome must be 0 or greater.'),
    amount: z
      .string()
      .min(1, 'A bet amount is required.')
      .regex(/^\d+(\.\d+)?$/, 'Amount must be a positive decimal string.'),
  })
  .loose();

// === Compile-time contract: each schema stays assignable to its generated type

// The generated types carry `email` as a plain `string` (the `format: email`
// annotation is a comment, not a TS brand), so a two-way assignability check is
// meaningful here.
type _SubscribeMatches = [
  z.infer<typeof newsletterSubscribeSchema> extends Schemas['NewsletterSubscribeRequest'] ? true : never,
  Schemas['NewsletterSubscribeRequest'] extends z.infer<typeof newsletterSubscribeSchema> ? true : never,
];
type _EmailRequestMatches = [
  z.infer<typeof emailRequestSchema> extends Schemas['EmailRequest'] ? true : never,
  Schemas['EmailRequest'] extends z.infer<typeof emailRequestSchema> ? true : never,
];
type _EmailTestMatches = [
  z.infer<typeof emailTestSchema> extends Schemas['EmailTestRequest'] ? true : never,
  Schemas['EmailTestRequest'] extends z.infer<typeof emailTestSchema> ? true : never,
];

// Referenced so `tsc --noEmit` evaluates them; `never` in any slot fails here.
export const __contract: [_SubscribeMatches, _EmailRequestMatches, _EmailTestMatches] = [
  [true, true],
  [true, true],
  [true, true],
];
