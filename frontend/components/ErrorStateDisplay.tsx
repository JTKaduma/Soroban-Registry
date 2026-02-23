'use client';

import { AlertCircle, RefreshCw } from 'lucide-react';
import { ApiError, NetworkError } from '@/lib/errors';

interface ErrorStateDisplayProps {
  error: Error;
  onRetry?: () => void;
  isRetrying?: boolean;
}

export default function ErrorStateDisplay({
  error,
  onRetry,
  isRetrying = false,
}: ErrorStateDisplayProps) {
  const isNetworkError = error instanceof NetworkError;
  const isApiError = error instanceof ApiError;

  const getErrorTitle = () => {
    if (isNetworkError) {
      return 'Connection Error';
    }
    if (isApiError && error.statusCode === 404) {
      return 'Not Found';
    }
    if (isApiError && error.statusCode === 401) {
      return 'Authentication Required';
    }
    if (isApiError && error.statusCode && error.statusCode >= 500) {
      return 'Server Error';
    }
    return 'Error';
  };

  const getSuggestion = () => {
    if (isNetworkError) {
      return 'Please check your internet connection and try again.';
    }
    if (isApiError && error.statusCode === 404) {
      return 'The resource you are looking for could not be found.';
    }
    if (isApiError && error.statusCode === 401) {
      return 'Please log in to access this resource.';
    }
    if (isApiError && error.statusCode && error.statusCode >= 500) {
      return 'Our servers are experiencing issues. Please try again in a moment.';
    }
    return 'An unexpected error occurred. Please try again.';
  };

  return (
    <div className="flex flex-col items-center justify-center p-8 text-center">
      <div className="mb-4 p-4 rounded-full bg-red-100 dark:bg-red-900/20">
        <AlertCircle className="w-12 h-12 text-red-600 dark:text-red-400" />
      </div>

      <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
        {getErrorTitle()}
      </h3>

      <p className="text-gray-600 dark:text-gray-400 mb-2 max-w-md">
        {error.message}
      </p>

      <p className="text-sm text-gray-500 dark:text-gray-500 mb-6 max-w-md">
        {getSuggestion()}
      </p>

      {onRetry && (
        <button
          onClick={onRetry}
          disabled={isRetrying}
          className="flex items-center gap-2 px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-400 text-white rounded-lg transition-colors font-medium"
          aria-label="Retry operation"
        >
          <RefreshCw className={`w-4 h-4 ${isRetrying ? 'animate-spin' : ''}`} />
          {isRetrying ? 'Retrying...' : 'Try Again'}
        </button>
      )}
    </div>
  );
}
