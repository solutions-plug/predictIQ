import express, { Application } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import dotenv from 'dotenv';
import { initSentry, Sentry } from './utils/sentry';
import logger from './utils/logger';
import { errorHandler } from './middleware/errorHandler';
import { requestLogger } from './middleware/requestLogger';
import { metricsMiddleware } from './middleware/metrics';
import healthRoutes from './routes/health';

// Load environment variables
dotenv.config();

// Initialize Sentry
initSentry();

const app: Application = express();
const PORT = process.env.PORT || 3000;

// Security middleware
app.use(helmet());
app.use(cors());

// Body parsing
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Sentry request handler (must be first)
app.use(Sentry.Handlers.requestHandler());
app.use(Sentry.Handlers.tracingHandler());

// Logging and metrics
app.use(requestLogger);
app.use(metricsMiddleware);

// Routes
app.use('/', healthRoutes);

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    error: {
      code: 'NOT_FOUND',
      message: 'Route not found',
      details: {},
      timestamp: new Date().toISOString(),
    },
  });
});

// Sentry error handler (must be before other error handlers)
app.use(Sentry.Handlers.errorHandler());

// Error handling middleware (must be last)
app.use(errorHandler);

// Graceful shutdown
process.on('SIGTERM', () => {
  logger.info('SIGTERM received, shutting down gracefully');
  process.exit(0);
});

process.on('SIGINT', () => {
  logger.info('SIGINT received, shutting down gracefully');
  process.exit(0);
});

// Unhandled rejection handler
process.on('unhandledRejection', (reason: any) => {
  logger.error({ reason }, 'Unhandled Promise Rejection');
  Sentry.captureException(reason);
});

// Uncaught exception handler
process.on('uncaughtException', (error: Error) => {
  logger.error({ error }, 'Uncaught Exception');
  Sentry.captureException(error);
  process.exit(1);
});

// Start server
app.listen(PORT, () => {
  logger.info(`Server running on port ${PORT} in ${process.env.NODE_ENV || 'development'} mode`);
});

export default app;
