'use client';

import { useQuery } from '@tanstack/react-query';
import { api, ContractSearchParams } from '@/lib/api';
import ContractCard from '@/components/ContractCard';
import { ActiveFilters } from '@/components/contracts/ActiveFilters';
import { FilterPanel } from '@/components/contracts/FilterPanel';
import { ResultsCount } from '@/components/contracts/ResultsCount';
import { SearchBar } from '@/components/contracts/SearchBar';
import { SortDropdown, SortBy } from '@/components/contracts/SortDropdown';
import { Filter, Package, SlidersHorizontal, X } from 'lucide-react';
import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { useEffect, useMemo, useState } from 'react';

const DEFAULT_PAGE_SIZE = 12;
const CATEGORY_OPTIONS = [
  'DeFi',
  'NFT',
  'Governance',
  'Infrastructure',
  'Payment',
  'Identity',
  'Gaming',
  'Social',
];
const LANGUAGE_OPTIONS = [
  'Rust',
  'TypeScript',
  'JavaScript',
  'AssemblyScript',
  'Move',
];

function parseCsvOrMulti(values: string[]) {
  return values
    .flatMap((value) => value.split(','))
    .map((value) => value.trim())
    .filter(Boolean);
}

function useDebouncedValue<T>(value: T, delay = 300) {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timeout = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timeout);
  }, [value, delay]);

  return debounced;
}

function removeOne<T>(values: T[], value: T) {
  return values.filter((current) => current !== value);
}

function toggleOne<T>(values: T[], value: T) {
  return values.includes(value)
    ? values.filter((current) => current !== value)
    : [...values, value];
}

type ContractsUiFilters = {
  query: string;
  categories: string[];
  languages: string[];
  author: string;
  networks: NonNullable<ContractSearchParams['network']>[];
  verified_only: boolean;
  sort_by: SortBy;
  page: number;
  page_size: number;
};

function getInitialFilters(searchParams: URLSearchParams): ContractsUiFilters {
  const query = searchParams.get('query') || searchParams.get('q') || '';
  const categories = parseCsvOrMulti(searchParams.getAll('category'));
  const languages = parseCsvOrMulti(searchParams.getAll('language'));
  const networks = parseCsvOrMulti(searchParams.getAll('network')).filter(
    (network): network is NonNullable<ContractSearchParams['network']> =>
      network === 'mainnet' || network === 'testnet' || network === 'futurenet',
  );
  const sortBy = searchParams.get('sort_by');
  const parsedPage = Number(searchParams.get('page') || '1');

  return {
    query,
    categories,
    languages,
    author: searchParams.get('author') || '',
    networks,
    verified_only: searchParams.get('verified_only') === 'true',
    sort_by:
      sortBy === 'name' ||
      sortBy === 'created_at' ||
      sortBy === 'popularity' ||
      sortBy === 'downloads'
        ? sortBy
        : 'created_at',
    page: Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1,
    page_size: DEFAULT_PAGE_SIZE,
  };
}

export function ContractsContent() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const [mobileFiltersOpen, setMobileFiltersOpen] = useState(false);

  const [filters, setFilters] = useState<ContractsUiFilters>(() =>
    getInitialFilters(new URLSearchParams(searchParams.toString())),
  );

  const debouncedQuery = useDebouncedValue(filters.query, 300);

  useEffect(() => {
    const params = new URLSearchParams();
    if (debouncedQuery) params.set('query', debouncedQuery);
    filters.categories.forEach((category) => params.append('category', category));
    filters.languages.forEach((language) => params.append('language', language));
    filters.networks.forEach((network) => params.append('network', network));
    if (filters.author) params.set('author', filters.author);
    if (filters.verified_only) params.set('verified_only', 'true');
    if (filters.sort_by !== 'created_at') params.set('sort_by', filters.sort_by);
    if (filters.page > 1) params.set('page', String(filters.page));
    params.set('page_size', String(filters.page_size));

    const next = params.toString();
    router.replace(next ? `${pathname}?${next}` : pathname, { scroll: false });
  }, [debouncedQuery, filters, pathname, router]);

  const apiParams = useMemo<ContractSearchParams>(
    () => ({
      query: debouncedQuery || undefined,
      categories: filters.categories.length > 0 ? filters.categories : undefined,
      languages: filters.languages.length > 0 ? filters.languages : undefined,
      author: filters.author || undefined,
      networks: filters.networks.length > 0 ? filters.networks : undefined,
      verified_only: filters.verified_only,
      sort_by: filters.sort_by,
      sort_order: filters.sort_by === 'name' ? 'asc' : 'desc',
      page: filters.page,
      page_size: filters.page_size,
    }),
    [debouncedQuery, filters],
  );

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ['contracts', apiParams],
    queryFn: () => api.getContracts(apiParams),
    placeholderData: (previousData) => previousData,
  });

  const clearAllFilters = () =>
    setFilters((current) => ({
      ...current,
      query: '',
      categories: [],
      languages: [],
      author: '',
      networks: [],
      verified_only: false,
      sort_by: 'created_at',
      page: 1,
    }));

  const activeFilterChips = useMemo(() => {
    const chips: Array<{ id: string; label: string; onRemove: () => void }> = [];

    if (debouncedQuery) {
      chips.push({
        id: 'query',
        label: `Search: ${debouncedQuery}`,
        onRemove: () => setFilters((current) => ({ ...current, query: '', page: 1 })),
      });
    }

    filters.categories.forEach((category) =>
      chips.push({
        id: `category:${category}`,
        label: `Category: ${category}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            categories: removeOne(current.categories, category),
            page: 1,
          })),
      }),
    );

    filters.languages.forEach((language) =>
      chips.push({
        id: `language:${language}`,
        label: `Language: ${language}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            languages: removeOne(current.languages, language),
            page: 1,
          })),
      }),
    );

    filters.networks.forEach((network) =>
      chips.push({
        id: `network:${network}`,
        label: `Network: ${network}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            networks: removeOne(current.networks, network),
            page: 1,
          })),
      }),
    );

    if (filters.author) {
      chips.push({
        id: 'author',
        label: `Author: ${filters.author}`,
        onRemove: () => setFilters((current) => ({ ...current, author: '', page: 1 })),
      });
    }

    if (filters.verified_only) {
      chips.push({
        id: 'verified',
        label: 'Verified only',
        onRemove: () =>
          setFilters((current) => ({ ...current, verified_only: false, page: 1 })),
      });
    }

    if (filters.sort_by !== 'created_at') {
      chips.push({
        id: 'sort',
        label: `Sort: ${filters.sort_by.replace('_', ' ')}`,
        onRemove: () => setFilters((current) => ({ ...current, sort_by: 'created_at' })),
      });
    }

    return chips;
  }, [debouncedQuery, filters]);

  const filterPanel = (
    <FilterPanel
      categories={CATEGORY_OPTIONS}
      selectedCategories={filters.categories}
      onToggleCategory={(value) =>
        setFilters((current) => ({
          ...current,
          categories: toggleOne(current.categories, value),
          page: 1,
        }))
      }
      languages={LANGUAGE_OPTIONS}
      selectedLanguages={filters.languages}
      onToggleLanguage={(value) =>
        setFilters((current) => ({
          ...current,
          languages: toggleOne(current.languages, value),
          page: 1,
        }))
      }
      selectedNetworks={filters.networks}
      onToggleNetwork={(value) =>
        setFilters((current) => ({
          ...current,
          networks: toggleOne(current.networks, value),
          page: 1,
        }))
      }
      author={filters.author}
      onAuthorChange={(value) =>
        setFilters((current) => ({ ...current, author: value, page: 1 }))
      }
      verifiedOnly={filters.verified_only}
      onVerifiedChange={(value) =>
        setFilters((current) => ({ ...current, verified_only: value, page: 1 }))
      }
    />
  );

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-8">
        <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-2">
          Browse Contracts
        </h1>
        <p className="text-gray-600 dark:text-gray-400">
          Discover verified Soroban smart contracts on the Stellar network
        </p>
      </div>

      <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 mb-8">
        <div className="flex flex-col gap-4">
          <SearchBar
            value={filters.query}
            onChange={(value) => setFilters((current) => ({ ...current, query: value, page: 1 }))}
            onClear={() => setFilters((current) => ({ ...current, query: '', page: 1 }))}
          />

          <div className="flex flex-wrap items-center gap-3">
            <SortDropdown
              value={filters.sort_by}
              onChange={(value) =>
                setFilters((current) => ({ ...current, sort_by: value, page: 1 }))
              }
            />
            <button
              type="button"
              onClick={() => setMobileFiltersOpen(true)}
              className="md:hidden inline-flex items-center gap-2 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-sm text-gray-700 dark:text-gray-300"
            >
              <SlidersHorizontal className="w-4 h-4" />
              Filters
            </button>
            <div className="hidden md:flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
              <Filter className="w-4 h-4" />
              Advanced filters
            </div>
            {isFetching && !isLoading && (
              <span className="text-xs text-gray-500 dark:text-gray-400">
                Updating results...
              </span>
            )}
          </div>

          <ActiveFilters chips={activeFilterChips} onClearAll={clearAllFilters} />
        </div>

        <div className="hidden md:block mt-6 border-t border-gray-200 dark:border-gray-800 pt-6">
          {filterPanel}
        </div>
      </div>

      {mobileFiltersOpen && (
        <div className="md:hidden fixed inset-0 z-50 bg-black/40">
          <div className="absolute right-0 top-0 h-full w-[88%] max-w-sm bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-800 p-5 overflow-y-auto">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Filters</h2>
              <button
                type="button"
                onClick={() => setMobileFiltersOpen(false)}
                className="p-1 rounded-md text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                aria-label="Close filters"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            {filterPanel}
            <button
              type="button"
              onClick={() => setMobileFiltersOpen(false)}
              className="mt-6 w-full px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700"
            >
              Show results
            </button>
          </div>
        </div>
      )}

      {isLoading ? (
        <div className="text-center py-12">
          <div className="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin" />
        </div>
      ) : data && data.items.length > 0 ? (
        <>
          <div className="mb-4">
            <ResultsCount visibleCount={data.items.length} totalCount={data.total} />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
            {data.items.map((contract) => (
              <ContractCard key={contract.id} contract={contract} />
            ))}
          </div>

          {data.total_pages > 1 && (
            <div className="flex items-center justify-center gap-2">
              <button
                onClick={() =>
                  setFilters((current) => ({ ...current, page: Math.max(1, current.page - 1) }))
                }
                disabled={filters.page <= 1}
                className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              >
                Previous
              </button>

              <span className="text-sm text-gray-600 dark:text-gray-400">
                Page {filters.page} of {data.total_pages}
              </span>

              <button
                onClick={() =>
                  setFilters((current) => ({ ...current, page: current.page + 1 }))
                }
                disabled={filters.page >= data.total_pages}
                className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              >
                Next
              </button>
            </div>
          )}
        </>
      ) : (
        <div className="text-center py-12 bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800">
          <Package className="w-12 h-12 text-gray-400 mx-auto mb-4" />
          <p className="text-gray-600 dark:text-gray-400 mb-4">
            No contracts found for the selected filters
          </p>
          <button
            type="button"
            onClick={clearAllFilters}
            className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            Clear all filters
          </button>
        </div>
      )}
    </div>
  );
}
