import { X } from 'lucide-react';

interface FilterChip {
  id: string;
  label: string;
  onRemove: () => void;
}

interface ActiveFiltersProps {
  chips: FilterChip[];
  onClearAll: () => void;
}

export function ActiveFilters({ chips, onClearAll }: ActiveFiltersProps) {
  if (chips.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap items-center gap-2 mt-4">
      {chips.map((chip) => (
        <button
          type="button"
          key={chip.id}
          onClick={chip.onRemove}
          className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs border border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-800 dark:bg-blue-950/50 dark:text-blue-300"
        >
          {chip.label}
          <X className="w-3 h-3" />
        </button>
      ))}
      <button
        type="button"
        onClick={onClearAll}
        className="text-xs px-2.5 py-1 rounded-full border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
      >
        Clear all filters
      </button>
    </div>
  );
}
