'use client';

import { useState, useCallback } from 'react';
import { useToast } from './useToast';
import { ApiError, NetworkError } from '@/lib/errors';

interface UseRetryOptions<T> {
  onSuccess?: (data: T) => void;
  onError?: (error: Error) => void;
  showToastOnError?: boolean;
}

interface UseRetryReturn<T, Args extends unknown[]> {
  execute: (...args: Args) => Promise<T | undefined>;
  retry: () => Promise<T | undefined>;
  isLoading: boolean;
  error: Error | null;
  data: T | null;
  reset: () => void;
}

export function useRetry<T, Args extends unknown[]>(
  asyncFunction: (...args: Args) => Promise<T>,
  options: UseRetryOptions<T> = {}
): UseRetryReturn<T, Args> {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<T | null>(null);
  const [lastArgs, setLastArgs] = useState<Args | null>(null);
  const { showError } = useToast();

  const execute = useCallback(
    async (...args: Args): Promise<T | undefined> => {
      setIsLoading(true);
      setError(null);
      setLastArgs(args);

      try {
        const result = await asyncFunction(...args);
        setData(result);
        setError(null);
        
        if (options.onSuccess) {
          options.onSuccess(result);
        }
        
        return result;
      } catch (err) {
        const error = err instanceof Error ? err : new Error('An unexpected error occurred');
        setError(error);
        setData(null);

        if (options.showToastOnError !== false) {
          let message = error.message;
          
          if (error instanceof NetworkError) {
            message = 'Network error. Please check your connection and try again.';
          } else if (error instanceof ApiError) {
            message = error.message;
          }
          
          showError(message);
        }

        if (options.onError) {
          options.onError(error);
        }

        return undefined;
      } finally {
        setIsLoading(false);
      }
    },
    [asyncFunction, options, showError]
  );

  const retry = useCallback(async (): Promise<T | undefined> => {
    if (!lastArgs) {
      console.warn('Cannot retry: no previous arguments stored');
      return undefined;
    }
    return execute(...lastArgs);
  }, [execute, lastArgs]);

  const reset = useCallback(() => {
    setIsLoading(false);
    setError(null);
    setData(null);
    setLastArgs(null);
  }, []);

  return {
    execute,
    retry,
    isLoading,
    error,
    data,
    reset,
  };
}
