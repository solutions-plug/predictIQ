import express, { Application } from 'express';
import cors from 'cors';
import pinoHttp from 'pino-http';
import { config } from './config';
import { logger } from './utils/logger';
import { errorHandler } from './middleware/errorHandler';
import { rateLimiter } from './middleware/rateLimiter';
import healthRoutes from './routes/health';
import authRoutes from './routes/auth';
import contentRoutes from './routes/content';

const app: Application = express();

app.use(cors({ origin: config.cors.origin, credentials: true }));
app.use(express.json());
app.use(pinoHttp({ logger }));
app.use(rateLimiter);

app.use('/health', healthRoutes);
app.use(`/api/${config.apiVersion}/auth`, authRoutes);
app.use(`/api/${config.apiVersion}/content`, contentRoutes);

app.use(errorHandler);

app.listen(config.port, () => {
  logger.info(`Server running on port ${config.port}`);
});

export default app;
