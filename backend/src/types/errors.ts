export enum ErrorCode {
  // Validation Errors (400)
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  INVALID_INPUT = 'INVALID_INPUT',
  
  // Authentication/Authorization Errors (401/403)
  UNAUTHORIZED = 'UNAUTHORIZED',
  FORBIDDEN = 'FORBIDDEN',
  
  // Resource Errors (404)
  NOT_FOUND = 'NOT_FOUND',
  MARKET_NOT_FOUND = 'MARKET_NOT_FOUND',
  
  // Business Logic Errors (409/422)
  MARKET_CLOSED = 'MARKET_CLOSED',
  INSUFFICIENT_BALANCE = 'INSUFFICIENT_BALANCE',
  ALREADY_VOTED = 'ALREADY_VOTED',
  
  // External Service Errors (502/503)
  BLOCKCHAIN_ERROR = 'BLOCKCHAIN_ERROR',
  ORACLE_FAILURE = 'ORACLE_FAILURE',
  
  // Server Errors (500)
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  DATABASE_ERROR = 'DATABASE_ERROR',
}

export interface ErrorDetails {
  [key: string]: any;
}

export class AppError extends Error {
  constructor(
    public code: ErrorCode,
    public message: string,
    public statusCode: number = 500,
    public details: ErrorDetails = {},
    public isOperational: boolean = true
  ) {
    super(message);
    Object.setPrototypeOf(this, AppError.prototype);
    Error.captureStackTrace(this, this.constructor);
  }
}

export interface ErrorResponse {
  error: {
    code: string;
    message: string;
    details: ErrorDetails;
    timestamp: string;
  };
}
