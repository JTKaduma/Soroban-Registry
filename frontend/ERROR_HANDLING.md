# Error Handling Documentation

This document describes the comprehensive error handling system implemented in the frontend application.

## Overview

The error handling system provides three layers of protection:

1. **Error Boundaries** - Catch React component errors
2. **API Error Handling** - Consistent error handling for all API calls
3. **Toast Notifications** - User-friendly notifications for transient errors

## Components

### ErrorBoundary

Catches JavaScript errors in React component trees and displays a fallback UI.

**Usage:**
```tsx
import ErrorBoundary from '@/components/ErrorBoundary';

<ErrorBoundary>
  <YourComponent />
</ErrorBoundary>
```

**Features:**
- Catches component errors and prevents app crashes
- Displays user-friendly error fallback UI
- Provides retry functionality
- Logs errors to console with component stack traces
- Supports optional external error logging

### Toast Notifications

Non-intrusive notifications for transient errors and status updates.

**Usage:**
```tsx
import { useToast } from '@/hooks/useToast';

function MyComponent() {
  const { showError, showSuccess, showWarning, showInfo } = useToast();
  
  const handleAction = async () => {
    try {
      await someApiCall();
      showSuccess('Operation completed successfully');
    } catch (error) {
      showError('Operation failed. Please try again.');
    }
  };
}
```

**Features:**
- Auto-dismiss after configurable timeout (default: 5000ms)
- Manual dismiss with close button
- Stack multiple toasts without overlapping
- Different visual styles for error types (error, warning, success, info)
- Accessible with ARIA labels and screen reader support

### API Error Handling

All API calls are wrapped with consistent error handling that:
- Catches network errors and connection issues
- Normalizes HTTP errors with user-friendly messages
- Extracts server-provided error messages
- Handles malformed JSON responses
- Maps status codes to appropriate messages

**Error Types:**
- `ApiError` - General API errors with status codes
- `NetworkError` - Connection and network issues
- `ValidationError` - Validation errors with field details

### Retry Mechanism

The `useRetry` hook provides retry functionality for failed operations.

**Usage:**
```tsx
import { useRetry } from '@/hooks/useRetry';
import { api } from '@/lib/api';

function MyComponent() {
  const { execute, retry, isLoading, error, data } = useRetry(
    (id: string) => api.getContract(id),
    {
      onSuccess: (data) => console.log('Success:', data),
      showToastOnError: true,
    }
  );
  
  useEffect(() => {
    execute('contract-id');
  }, []);
  
  if (error) {
    return <ErrorStateDisplay error={error} onRetry={retry} isRetrying={isLoading} />;
  }
  
  return <div>{/* render data */}</div>;
}
```

**Features:**
- Automatic retry with same parameters
- Loading state management
- Error state management
- Optional toast notifications
- Success/error callbacks

### ErrorStateDisplay

Reusable component for displaying error states with retry functionality.

**Usage:**
```tsx
import ErrorStateDisplay from '@/components/ErrorStateDisplay';

<ErrorStateDisplay 
  error={error} 
  onRetry={handleRetry} 
  isRetrying={isLoading} 
/>
```

**Features:**
- User-friendly error messages
- Actionable suggestions based on error type
- Retry button with loading state
- Responsive design
- Accessible with ARIA labels

## Error Messages

The system provides user-friendly error messages for common HTTP status codes:

- **400** - Invalid request. Please check your input.
- **401** - Authentication required. Please log in.
- **403** - You do not have permission to perform this action.
- **404** - The requested resource was not found.
- **409** - This action conflicts with existing data.
- **422** - The provided data is invalid.
- **429** - Too many requests. Please try again later.
- **500** - A server error occurred. Please try again.
- **502** - The server is temporarily unavailable.
- **503** - The service is temporarily unavailable.
- **504** - The request timed out. Please try again.

Network errors display: "Unable to connect to the server. Please check your internet connection."

## Accessibility

All error handling components follow accessibility best practices:

- **ARIA Labels** - All interactive elements have appropriate ARIA labels
- **Keyboard Navigation** - All buttons and interactive elements are keyboard accessible
- **Screen Reader Support** - Toast notifications use `role="alert"` for screen readers
- **Focus Management** - Proper focus management when errors occur
- **Color Contrast** - All text meets WCAG AA standards for color contrast

## Best Practices

### 1. Use Error Boundaries for Component Errors

Wrap sections of your app that might throw errors:

```tsx
<ErrorBoundary>
  <ComplexFeature />
</ErrorBoundary>
```

### 2. Use Toast Notifications for Transient Errors

Show non-intrusive notifications for temporary issues:

```tsx
const { showError } = useToast();

try {
  await api.someAction();
} catch (error) {
  showError('Action failed. Please try again.');
}
```

### 3. Use ErrorStateDisplay for Inline Errors

Display errors inline with retry functionality:

```tsx
if (error) {
  return <ErrorStateDisplay error={error} onRetry={retry} />;
}
```

### 4. Use useRetry for Operations That Can Be Retried

Wrap API calls that users might want to retry:

```tsx
const { execute, retry, error, isLoading } = useRetry(
  () => api.fetchData(id)
);
```

### 5. Let API Errors Propagate

The API client automatically handles errors, so you can catch them at the component level:

```tsx
try {
  const data = await api.getContract(id);
  // handle success
} catch (error) {
  // error is already normalized and user-friendly
  if (error instanceof NetworkError) {
    // handle network error
  } else if (error instanceof ApiError) {
    // handle API error
  }
}
```

## Testing

The error handling system is designed to be testable:

- Mock API errors by throwing errors in tests
- Simulate component errors for Error Boundary testing
- Test toast notifications with different types
- Verify retry functionality with mock functions

## Future Enhancements

Potential improvements for the error handling system:

- Integration with external error tracking (Sentry, LogRocket)
- Offline detection and automatic retry with exponential backoff
- Error analytics and reporting dashboard
- Customizable error messages per environment
- Per-route error boundaries for better isolation
- Undo/redo functionality for recoverable errors
