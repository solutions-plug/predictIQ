import { Request, Response, NextFunction } from 'express';
import logger from '../utils/logger';

const SLOW_QUERY_THRESHOLD = 1000; // 1 second

export function requestLogger(req: Request, res: Response, next: NextFunction) {
  const start = Date.now();

  res.on('finish', () => {
    const duration = Date.now() - start;
    const logData = {
      method: req.method,
      url: req.url,
      statusCode: res.statusCode,
      duration,
      userAgent: req.get('user-agent'),
      ip: req.ip,
    };

    if (duration > SLOW_QUERY_THRESHOLD) {
      logger.warn(logData, 'Slow request detected');
    } else {
      logger.info(logData, 'Request completed');
    }
  });

  next();
}
