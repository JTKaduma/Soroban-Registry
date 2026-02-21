import { MaturityLevel } from '@/lib/api';

interface MaturityBadgeProps {
  level: MaturityLevel;
  size?: 'sm' | 'md' | 'lg';
}

const maturityConfig = {
  alpha: {
    label: 'Alpha',
    color: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300',
    description: 'Experimental - Use with caution',
  },
  beta: {
    label: 'Beta',
    color: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300',
    description: 'Testing phase - Feedback welcome',
  },
  stable: {
    label: 'Stable',
    color: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300',
    description: 'Production ready',
  },
  mature: {
    label: 'Mature',
    color: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300',
    description: 'Battle-tested and reliable',
  },
  legacy: {
    label: 'Legacy',
    color: 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300',
    description: 'Deprecated - Migration recommended',
  },
};

export default function MaturityBadge({ level, size = 'md' }: MaturityBadgeProps) {
  const config = maturityConfig[level];
  const sizeClasses = {
    sm: 'text-xs px-2 py-0.5',
    md: 'text-sm px-2.5 py-1',
    lg: 'text-base px-3 py-1.5',
  };

  return (
    <span
      className={`inline-flex items-center rounded-full font-medium ${config.color} ${sizeClasses[size]}`}
      title={config.description}
    >
      {config.label}
    </span>
  );
}
