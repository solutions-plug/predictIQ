import { Request, Response, NextFunction } from 'express';
import { logger } from '../utils/logger';

export class AppError extends Error {
  statusCode: number;
  isOperational: boolean;

  constructor(message: string, statusCode: number) {
    super(message);
    this.statusCode = statusCode;
    this.isOperational = true;
    Error.captureStackTrace(this, this.constructor);
  }
}

export const errorHandler = (
  err: Error | AppError,
  req: Request,
  res: Response,
  next: NextFunction
) => {
  if (err instanceof AppError) {
    logger.error({ err, path: req.path }, 'Operational error');
    return res.status(err.statusCode).json({
      status: 'error',
      message: err.message,
    });
  }

  logger.error({ err, path: req.path }, 'Unexpected error');
  res.status(500).json({
    status: 'error',
    message: 'Internal server error',
  });
};
