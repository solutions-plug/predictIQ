import { Router, Request, Response } from 'express';
import { register } from '../utils/metrics';
import logger from '../utils/logger';

const router = Router();

// Basic health check
router.get('/health', (req: Request, res: Response) => {
  res.status(200).json({
    status: 'ok',
    timestamp: new Date().toISOString(),
    uptime: process.uptime(),
  });
});

// Readiness check (checks dependencies)
router.get('/health/ready', async (req: Request, res: Response) => {
  const checks = {
    server: true,
    // Add dependency checks here (database, blockchain, etc.)
  };

  const isReady = Object.values(checks).every(check => check === true);
  const status = isReady ? 200 : 503;

  res.status(status).json({
    status: isReady ? 'ready' : 'not ready',
    checks,
    timestamp: new Date().toISOString(),
  });
});

// Liveness check
router.get('/health/live', (req: Request, res: Response) => {
  res.status(200).json({
    status: 'alive',
    timestamp: new Date().toISOString(),
  });
});

// Metrics endpoint
router.get('/metrics', async (req: Request, res: Response) => {
  try {
    res.set('Content-Type', register.contentType);
    const metrics = await register.metrics();
    res.send(metrics);
  } catch (err) {
    logger.error({ err }, 'Error collecting metrics');
    res.status(500).send('Error collecting metrics');
  }
});

export default router;
