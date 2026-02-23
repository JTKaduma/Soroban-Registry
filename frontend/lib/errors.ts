/**
 * Custom error classes for API and network errors
 */

export class ApiError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public originalError?: unknown,
    public endpoint?: string
  ) {
    super(message);
    this.name = 'ApiError';
    Object.setPrototypeOf(this, ApiError.prototype);
  }
}

export class NetworkError extends ApiError {
  constructor(message: string, endpoint?: string) {
    super(message, undefined, undefined, endpoint);
    this.name = 'NetworkError';
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

export class ValidationError extends ApiError {
  constructor(
    message: string,
    public fields?: Record<string, string[]>
  ) {
    super(message, 400);
    this.name = 'ValidationError';
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Error message mapping for HTTP status codes
 */
export function getErrorMessage(statusCode: number, serverMessage?: string): string {
  if (serverMessage) return serverMessage;
  
  const messages: Record<number, string> = {
    400: 'Invalid request. Please check your input.',
    401: 'Authentication required. Please log in.',
    403: 'You do not have permission to perform this action.',
    404: 'The requested resource was not found.',
    409: 'This action conflicts with existing data.',
    422: 'The provided data is invalid.',
    429: 'Too many requests. Please try again later.',
    500: 'A server error occurred. Please try again.',
    502: 'The server is temporarily unavailable.',
    503: 'The service is temporarily unavailable.',
    504: 'The request timed out. Please try again.',
  };
  
  return messages[statusCode] || 'An unexpected error occurred.';
}

/**
 * Extract error data from API response
 */
export async function extractErrorData(response: Response): Promise<{ message?: string; details?: unknown }> {
  try {
    const contentType = response.headers.get('content-type');
    if (contentType?.includes('application/json')) {
      const data = await response.json();
      return {
        message: data.message || data.error || data.detail,
        details: data,
      };
    }
    const text = await response.text();
    return { message: text };
  } catch {
    return {};
  }
}

/**
 * Create appropriate error based on status code and error data
 */
export function createApiError(
  statusCode: number,
  errorData: { message?: string; details?: unknown },
  endpoint: string
): ApiError {
  const message = getErrorMessage(statusCode, errorData.message);
  
  if (statusCode === 422 && errorData.details && typeof errorData.details === 'object') {
    const details = errorData.details as Record<string, unknown>;
    if (details.fields) {
      return new ValidationError(message, details.fields as Record<string, string[]>);
    }
  }
  
  return new ApiError(message, statusCode, errorData.details, endpoint);
}

/**
 * Normalize any error into a consistent structure
 */
export interface NormalizedError {
  message: string;
  statusCode?: number;
  type: 'network' | 'api' | 'validation' | 'unknown';
  endpoint?: string;
  timestamp: string;
  details?: unknown;
}

export function normalizeError(error: unknown, endpoint?: string): NormalizedError {
  const timestamp = new Date().toISOString();
  
  if (error instanceof NetworkError) {
    return {
      message: error.message,
      type: 'network',
      endpoint: error.endpoint || endpoint,
      timestamp,
    };
  }
  
  if (error instanceof ValidationError) {
    return {
      message: error.message,
      statusCode: error.statusCode,
      type: 'validation',
      endpoint: error.endpoint || endpoint,
      timestamp,
      details: error.fields,
    };
  }
  
  if (error instanceof ApiError) {
    return {
      message: error.message,
      statusCode: error.statusCode,
      type: 'api',
      endpoint: error.endpoint || endpoint,
      timestamp,
      details: error.originalError,
    };
  }
  
  if (error instanceof Error) {
    return {
      message: error.message,
      type: 'unknown',
      endpoint,
      timestamp,
      details: error,
    };
  }
  
  return {
    message: 'An unexpected error occurred',
    type: 'unknown',
    endpoint,
    timestamp,
    details: error,
  };
}

/**
 * Error logging utility
 */
export interface ErrorLogger {
  logError(error: NormalizedError): void;
}

let errorLogger: ErrorLogger | null = null;

export function setErrorLogger(logger: ErrorLogger | null) {
  errorLogger = logger;
}

export function logError(error: Error, context?: Record<string, unknown>) {
  console.error('[Error]', {
    timestamp: new Date().toISOString(),
    message: error.message,
    name: error.name,
    stack: error.stack,
    ...context,
  });
  
  if (errorLogger) {
    const normalized = normalizeError(error, context?.endpoint as string);
    errorLogger.logError(normalized);
  }
}
