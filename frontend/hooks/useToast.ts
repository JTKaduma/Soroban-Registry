'use client';

import { useContext } from 'react';
import { ToastContext, ToastContextValue } from '@/providers/ToastProvider';

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  
  return context;
}
