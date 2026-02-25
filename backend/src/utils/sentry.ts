import * as Sentry from '@sentry/node';
import logger from './logger';

export function initSentry() {
  if (process.env.SENTRY_DSN) {
    Sentry.init({
      dsn: process.env.SENTRY_DSN,
      environment: process.env.SENTRY_ENVIRONMENT || 'development',
      tracesSampleRate: 1.0,
      integrations: [
        new Sentry.Integrations.Http({ tracing: true }),
      ],
    });
    logger.info('Sentry initialized');
  } else {
    logger.warn('Sentry DSN not configured, error tracking disabled');
  }
}

export { Sentry };
