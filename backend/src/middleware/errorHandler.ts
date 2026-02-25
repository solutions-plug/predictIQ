import { Request, Response, NextFunction } from 'express';
import { AppError, ErrorCode, ErrorResponse } from '../types/errors';
import logger from '../utils/logger';
import { Sentry } from '../utils/sentry';

export function errorHandler(
  err: Error | AppError,
  req: Request,
  res: Response,
  next: NextFunction
) {
  // Default error values
  let statusCode = 500;
  let code = ErrorCode.INTERNAL_ERROR;
  let message = 'An unexpected error occurred';
  let details = {};

  // Handle AppError instances
  if (err instanceof AppError) {
    statusCode = err.statusCode;
    code = err.code;
    message = err.message;
    details = err.details;

    // Log operational errors as warnings
    if (err.isOperational) {
      logger.warn({ err, req: { method: req.method, url: req.url } }, message);
    } else {
      logger.error({ err, req: { method: req.method, url: req.url } }, message);
      Sentry.captureException(err);
    }
  } else {
    // Handle unexpected errors
    logger.error({ err, req: { method: req.method, url: req.url } }, 'Unexpected error');
    Sentry.captureException(err);
  }

  // Build error response
  const errorResponse: ErrorResponse = {
    error: {
      code,
      message,
      details,
      timestamp: new Date().toISOString(),
    },
  };

  // Don't leak error details in production
  if (process.env.NODE_ENV === 'production' && statusCode === 500) {
    errorResponse.error.message = 'Internal server error';
    errorResponse.error.details = {};
  }

  res.status(statusCode).json(errorResponse);
}

// Async error wrapper
export const asyncHandler = (fn: Function) => (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  Promise.resolve(fn(req, res, next)).catch(next);
};
