import express, { Application } from 'express';
import cors from 'cors';
import pinoHttp from 'pino-http';
import { config } from './config';
import { connectDatabase } from './config/database';
import { logger } from './utils/logger';
import { errorHandler } from './middleware/errorHandler';
import { rateLimiter } from './middleware/rateLimiter';
import healthRoutes from './routes/health';
import landingRoutes from './routes/landing';

const app: Application = express();

// Middleware
app.use(cors({ origin: config.cors.origin, credentials: true }));
app.use(express.json());
app.use(pinoHttp({ logger }));
app.use(rateLimiter);

// Routes
app.use('/health', healthRoutes);
app.use(`/api/${config.apiVersion}`, landingRoutes);

// Error handling
app.use(errorHandler);

const startServer = async () => {
  try {
    await connectDatabase();
    
    app.listen(config.port, () => {
      logger.info(`Server running on port ${config.port} in ${config.env} mode`);
      logger.info(`API version: ${config.apiVersion}`);
    });
  } catch (error) {
    logger.error('Failed to start server:', error);
    process.exit(1);
  }
};

startServer();

export default app;
