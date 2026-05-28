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
});

export type EnvConfig = z.infer<typeof envSchema>;

/**
 * Validate environment variables against the schema.
 * Throws with a descriptive message listing every missing or invalid variable.
 */
export function validateEnvironment(): EnvConfig {
  const result = envSchema.safeParse({
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
  });

  if (!result.success) {
    const issues = result.error.errors
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

// Validate once at module load so any import of this file fails fast —
// on the server during `next build` this surfaces missing vars as a build error.
export const env = validateEnvironment();
