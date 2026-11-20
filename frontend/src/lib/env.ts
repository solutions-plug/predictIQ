/**
 * Environment variable validation for frontend.
 * Uses zod to validate required variables at build time so the app never
 * starts with a missing NEXT_PUBLIC_API_URL and returns a cryptic runtime error.
 */

import { z } from 'zod';

const envSchema = z.object({
  NEXT_PUBLIC_API_URL: z
    .string()
    .min(1, 'NEXT_PUBLIC_API_URL must not be empty')
    .url('NEXT_PUBLIC_API_URL must be a valid URL'),
  // Optional: base URL for services/tts. Unset disables TTS features rather
  // than failing the build, since not every deployment runs the TTS service.
  NEXT_PUBLIC_TTS_API_URL: z
    .string()
    .url('NEXT_PUBLIC_TTS_API_URL must be a valid URL')
    .optional()
    .or(z.literal('')),
});

export type EnvConfig = z.infer<typeof envSchema>;

/**
 * Validate environment variables against the schema.
 * Throws with a descriptive message listing every missing or invalid variable.
 */
export function validateEnvironment(): EnvConfig {
  const result = envSchema.safeParse({
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
    NEXT_PUBLIC_TTS_API_URL: process.env.NEXT_PUBLIC_TTS_API_URL,
  });

  if (!result.success) {
    const issues = result.error.issues
      .map((e) => `  - ${e.path.join('.')}: ${e.message}`)
      .join('\n');
    throw new Error(`Missing or invalid environment variables:\n${issues}`);
  }

  return result.data;
}

/**
 * Get validated environment configuration.
 * Re-uses the cached result from module initialisation.
 */
export function getEnvConfig(): EnvConfig {
  return env;
}

/**
 * Resolve the API origin used for admin auth-gate validation.
 * Falls back to the validated NEXT_PUBLIC_API_URL and strips any trailing
 * slash so callers can safely append `/api/v1/...` paths.
 */
export function getApiOrigin(): string {
  return env.NEXT_PUBLIC_API_URL.replace(/\/+$/, '');
}

/**
 * Build an absolute API URL for a given path (e.g. `/api/v1/markets/1/claim`).
 * Resolves against the configured API origin so requests never hit the
 * Next.js host, and normalises the leading slash on the path.
 */
export function getApiUrl(path: string): string {
  const normalised = path.startsWith('/') ? path : `/${path}`;
  return `${getApiOrigin()}${normalised}`;
}

/**
 * Build the absolute API URL for a market dispute submission.
 * Mirrors the real dispute route in services/api/openapi.yaml
 * (`POST /api/v1/markets/{market_id}/disputes`) so the dispute flow never
 * targets the nonexistent `/blockchain/markets/{id}/disputes` endpoint.
 */
export function getMarketDisputeUrl(marketId: string | number): string {
  return getApiUrl(`/api/v1/markets/${encodeURIComponent(String(marketId))}/disputes`);
}

// Validate once at module load so any import of this file fails fast —
// on the server during `next build` this surfaces missing vars as a build error.
export const env = validateEnvironment();
