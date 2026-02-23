export type SortBy = 'name' | 'created_at' | 'updated_at' | 'popularity' | 'deployments' | 'interactions' | 'relevance' | 'downloads';

interface SortDropdownProps {
  value: SortBy;
  onChange: (value: SortBy) => void;
  showRelevance?: boolean;
}

const SORT_OPTIONS: { value: SortBy; label: string }[] = [
  { value: 'created_at', label: 'Newest First' },
  { value: 'updated_at', label: 'Recently Updated' },
  { value: 'popularity', label: 'Most Popular' },
  { value: 'deployments', label: 'Most Deployed' },
  { value: 'interactions', label: 'Most Interactions' },
  { value: 'downloads', label: 'Downloads' },
  { value: 'name', label: 'Name' },
];

export function SortDropdown({ value, onChange, showRelevance }: SortDropdownProps) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as SortBy)}
      className="px-3 py-2 rounded-lg border border-border bg-background text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20"
      aria-label="Sort contracts"
    >
      {SORT_OPTIONS.map((option) => (
        <option key={option.value} value={option.value}>
          Sort: {option.label}
        </option>
      ))}
      {showRelevance && (
        <option value="relevance">Sort: Relevance</option>
      )}
    </select>
  );
}
