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
  const issues = envResult.error.errors
    .map((e) => `  - ${e.path.join('.')}: ${e.message}`)
    .join('\n');
  throw new Error(`\nMissing or invalid environment variables:\n${issues}\n`);
}

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Enable code splitting and optimization
  swcMinify: true,
  
  // Optimize bundle size
  webpack: (config, { isServer }) => {
    config.optimization = {
      ...config.optimization,
      splitChunks: {
        chunks: 'all',
        cacheGroups: {
          default: false,
          vendors: false,
          // Vendor chunk
          vendor: {
            filename: 'chunks/vendor.js',
            test: /node_modules/,
            priority: 10,
            reuseExistingChunk: true,
            enforce: true,
          },
          // Common chunk
          common: {
            minChunks: 2,
            priority: 5,
            reuseExistingChunk: true,
            filename: 'chunks/common.js',
          },
        },
      },
    };
    return config;
  },

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
            key: 'Content-Security-Policy',
            value: "default-src 'self'; script-src 'self' 'strict-dynamic'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' https:; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; upgrade-insecure-requests",
          },
        ],
      },
    ];
  },
};

module.exports = nextConfig;
