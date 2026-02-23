'use client';

import { Check, Copy } from 'lucide-react';

interface CodeCopyButtonProps {
  onCopy: () => void;
  copied: boolean;
  disabled?: boolean;
}

export default function CodeCopyButton({
  onCopy,
  copied,
  disabled = false,
}: CodeCopyButtonProps) {
  return (
    <button
      onClick={onCopy}
      disabled={disabled}
      // Button is intentionally presentation-only; copy logic/analytics live in useCopy.
      className="inline-flex items-center gap-1 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 px-2.5 py-1 text-xs font-medium text-gray-700 dark:text-gray-200 transition-colors hover:bg-gray-50 dark:hover:bg-gray-800 disabled:cursor-not-allowed disabled:opacity-60"
      aria-label={copied ? 'Code copied' : 'Copy code'}
      title={copied ? 'Copied' : 'Copy code'}
    >
      {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      {copied ? 'Copied' : 'Copy'}
    </button>
  );
}
