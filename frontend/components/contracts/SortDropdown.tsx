export type SortBy = 'name' | 'created_at' | 'popularity' | 'downloads';

interface SortDropdownProps {
  value: SortBy;
  onChange: (value: SortBy) => void;
}

const SORT_OPTIONS: { value: SortBy; label: string }[] = [
  { value: 'name', label: 'Name' },
  { value: 'created_at', label: 'Date Created' },
  { value: 'popularity', label: 'Popularity' },
  { value: 'downloads', label: 'Downloads' },
];

export function SortDropdown({ value, onChange }: SortDropdownProps) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as SortBy)}
      className="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-300"
      aria-label="Sort contracts"
    >
      {SORT_OPTIONS.map((option) => (
        <option key={option.value} value={option.value}>
          Sort: {option.label}
        </option>
      ))}
    </select>
  );
}
