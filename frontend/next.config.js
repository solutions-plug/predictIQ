// Validate required environment variables at build time.
// If any are missing the build aborts with a clear error rather than
// producing a bundle that fails silently at runtime.
const { z } = require('zod');

const envSchema = z.object({
  NEXT_PUBLIC_API_URL: z
    .string()
    .min(1, 'NEXT_PUBLIC_API_URL must not be empty')
    .url('NEXT_PUBLIC_API_URL must be a valid URL'),
});

const envResult = envSchema.safeParse({
  NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
});

if (!envResult.success) {
  const issues = envResult.error.issues
    .map((e) => `  - ${e.path.join('.')}: ${e.message}`)
    .join('\n');
  throw new Error(`\nMissing or invalid environment variables:\n${issues}\n`);
}

const withBundleAnalyzer = require('@next/bundle-analyzer')({
  enabled: process.env.ANALYZE === 'true',
});

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Next.js 16 uses Turbopack by default, which handles chunk splitting
  // and minification automatically (no swcMinify / custom webpack needed).

  // Enable experimental features for better performance
  experimental: {
    optimizePackageImports: ['react', 'react-dom'],
  },

  // Compress responses
  compress: true,

  // Generate ETags for caching
  generateEtags: true,

  // Optimize images
  images: {
    formats: ['image/avif', 'image/webp'],
  },

  // Security headers
  async headers() {
    return [
      {
        source: '/:path*',
        headers: [
          {
            key: 'X-Content-Type-Options',
            value: 'nosniff',
          },
          {
            key: 'X-Frame-Options',
            value: 'DENY',
          },
          {
            key: 'Referrer-Policy',
            value: 'strict-origin-when-cross-origin',
          },
          {
            key: 'Permissions-Policy',
            value: 'geolocation=(), microphone=(), camera=()',
          },
          {
            key: 'Strict-Transport-Security',
            value: 'max-age=63072000; includeSubDomains; preload',
          },
          // Content-Security-Policy is intentionally NOT set here. It is
          // computed per-request in src/proxy.ts, which needs a fresh nonce
          // and the runtime API origin for connect-src — values this static
          // config cannot supply. Defining it in both places would mean two
          // divergent policies write the same header for the same routes.
        ],
      },
    ];
  },
};

module.exports = withBundleAnalyzer(nextConfig);
