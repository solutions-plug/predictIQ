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
 * Resolve a path against the configured API origin.
 * Admin pages must not use relative URLs: those resolve against the Next.js
 * origin and are rejected by the API. This accessor keeps the base URL in one
 * place instead of reading process.env inline.
 */
export function getApiUrl(path: string): string {
  const base = env.NEXT_PUBLIC_API_URL.replace(/\/+$/, '');
  const suffix = path.startsWith('/') ? path : `/${path}`;
  return `${base}${suffix}`;
}

/**
 * Build the auth headers for admin API requests, reusing the admin API key
 * established for the admin gate. Returns an empty object when no key is set
 * so callers can spread it unconditionally.
 */
export function getAdminAuthHeaders(): Record<string, string> {
  const apiKey = process.env.NEXT_PUBLIC_ADMIN_API_KEY;
  return apiKey ? { 'X-Admin-API-Key': apiKey } : {};
}

// Validate once at module load so any import of this file fails fast —
// on the server during `next build` this surfaces missing vars as a build error.
export const env = validateEnvironment();
