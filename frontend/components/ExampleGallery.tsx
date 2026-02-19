'use client';

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api';
import ExampleCard from './ExampleCard';
import { AlertCircle, Terminal, Search } from 'lucide-react';

interface ExampleGalleryProps {
  contractId: string;
}

export default function ExampleGallery({ contractId }: ExampleGalleryProps) {
  const { data: examples, isLoading, error } = useQuery({
    queryKey: ['contract-examples', contractId],
    queryFn: () => api.getContractExamples(contractId),
  });

  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  if (isLoading) {
    return <div className="animate-pulse h-64 bg-gray-100 dark:bg-gray-800 rounded-xl" />;
  }

  if (error) {
    return (
      <div className="p-4 bg-red-50 text-red-600 rounded-lg flex items-center gap-2">
        <AlertCircle className="w-5 h-5" />
        Failed to load examples
      </div>
    );
  }

  if (!examples || examples.length === 0) {
    return (
      <div className="text-center py-12 bg-gray-50 dark:bg-gray-900/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
        <Terminal className="w-12 h-12 text-gray-400 mx-auto mb-4" />
        <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
          No Examples Yet
        </h3>
        <p className="text-gray-500 dark:text-gray-400 max-w-sm mx-auto">
          There are no code examples for this contract yet. Be the first to contribute one!
        </p>
      </div>
    );
  }

  const filteredExamples = examples.filter(e => {
    const matchesCategory = selectedCategory === 'all' || e.category === selectedCategory;
    const matchesSearch = !searchQuery.trim() || 
      e.title.toLowerCase().includes(searchQuery.toLowerCase()) || 
      e.description?.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const categories = ['all', 'basic', 'advanced', 'integration'];

  return (
    <div className="space-y-8">
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Usage Examples
            </h2>
            <span className="px-2 py-1 rounded-full bg-gray-100 dark:bg-gray-800 text-xs font-medium text-gray-600 dark:text-gray-400">
              {filteredExamples.length}
            </span>
          </div>
        </div>

        <div className="flex flex-col sm:flex-row gap-4 justify-between">
          <div className="relative max-w-md w-full">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search examples by title or description..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-9 pr-4 py-2 rounded-lg border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 text-sm focus:ring-2 focus:ring-blue-500 outline-none transition-all"
            />
          </div>

          <div className="flex p-1 bg-gray-100 dark:bg-gray-800 rounded-lg overflow-x-auto">
            {categories.map((cat) => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-4 py-2 rounded-md text-sm font-medium transition-all capitalize whitespace-nowrap ${
                  selectedCategory === cat
                    ? 'bg-white dark:bg-gray-700 shadow-sm text-gray-900 dark:text-white'
                    : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
                }`}
              >
                {cat}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-8">
        {filteredExamples.length > 0 ? (
          filteredExamples.map((example) => (
            <ExampleCard key={example.id} example={example} />
          ))
        ) : (
          <div className="text-center py-12 bg-gray-50 dark:bg-gray-900/50 rounded-xl">
            <p className="text-gray-500 dark:text-gray-400">
              No examples found matching your criteria.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
