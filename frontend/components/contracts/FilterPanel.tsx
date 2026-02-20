import { ContractSearchParams } from '@/lib/api';

type NetworkFilter = NonNullable<ContractSearchParams['network']>;

interface FilterPanelProps {
  categories: string[];
  selectedCategories: string[];
  onToggleCategory: (value: string) => void;
  languages: string[];
  selectedLanguages: string[];
  onToggleLanguage: (value: string) => void;
  selectedNetworks: NetworkFilter[];
  onToggleNetwork: (value: NetworkFilter) => void;
  author: string;
  onAuthorChange: (value: string) => void;
  verifiedOnly: boolean;
  onVerifiedChange: (value: boolean) => void;
}

function CheckboxGroup({
  title,
  options,
  selected,
  onToggle,
}: {
  title: string;
  options: string[];
  selected: string[];
  onToggle: (value: string) => void;
}) {
  return (
    <div>
      <p className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{title}</p>
      <div className="space-y-2">
        {options.map((option) => (
          <label key={option} className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={selected.includes(option)}
              onChange={() => onToggle(option)}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            />
            <span className="text-sm text-gray-700 dark:text-gray-300">{option}</span>
          </label>
        ))}
      </div>
    </div>
  );
}

export function FilterPanel({
  categories,
  selectedCategories,
  onToggleCategory,
  languages,
  selectedLanguages,
  onToggleLanguage,
  selectedNetworks,
  onToggleNetwork,
  author,
  onAuthorChange,
  verifiedOnly,
  onVerifiedChange,
}: FilterPanelProps) {
  const networks: NetworkFilter[] = ['mainnet', 'testnet', 'futurenet'];

  return (
    <div className="space-y-5">
      <CheckboxGroup
        title="Category"
        options={categories}
        selected={selectedCategories}
        onToggle={onToggleCategory}
      />

      <CheckboxGroup
        title="Language"
        options={languages}
        selected={selectedLanguages}
        onToggle={onToggleLanguage}
      />

      <div>
        <p className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Network</p>
        <div className="space-y-2">
          {networks.map((network) => (
            <label key={network} className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={selectedNetworks.includes(network)}
                onChange={() => onToggleNetwork(network)}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              />
              <span className="text-sm capitalize text-gray-700 dark:text-gray-300">
                {network}
              </span>
            </label>
          ))}
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          Author
        </label>
        <input
          type="text"
          value={author}
          onChange={(e) => onAuthorChange(e.target.value)}
          placeholder="Publisher username or address"
          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={verifiedOnly}
          onChange={(e) => onVerifiedChange(e.target.checked)}
          className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
        />
        <span className="text-sm text-gray-700 dark:text-gray-300">Verified only</span>
      </label>
    </div>
  );
}
