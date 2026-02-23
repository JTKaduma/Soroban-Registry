'use client';

import { useState } from 'react';
import ContractCardSkeleton from './ContractCardSkeleton';
import ExampleCardSkeleton from './ExampleCardSkeleton';
import TemplateCardSkeleton from './TemplateCardSkeleton';

/**
 * Demo component to showcase skeleton loaders
 * This can be used for testing and documentation purposes
 */
export default function SkeletonDemo() {
  const [activeTab, setActiveTab] = useState<'contract' | 'example' | 'template'>('contract');

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      <div className="mb-8">
        <h2 className="text-2xl font-bold mb-4">Skeleton Loader Demo</h2>
        <p className="text-gray-600 dark:text-gray-400 mb-6">
          Preview of loading states for different card types
        </p>

        <div className="flex gap-2 mb-6">
          <button
            onClick={() => setActiveTab('contract')}
            className={`px-4 py-2 rounded-lg font-medium transition-colors ${
              activeTab === 'contract'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
            }`}
          >
            Contract Card
          </button>
          <button
            onClick={() => setActiveTab('example')}
            className={`px-4 py-2 rounded-lg font-medium transition-colors ${
              activeTab === 'example'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
            }`}
          >
            Example Card
          </button>
          <button
            onClick={() => setActiveTab('template')}
            className={`px-4 py-2 rounded-lg font-medium transition-colors ${
              activeTab === 'template'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
            }`}
          >
            Template Card
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {activeTab === 'contract' && (
          <>
            <ContractCardSkeleton />
            <ContractCardSkeleton />
            <ContractCardSkeleton />
          </>
        )}
        {activeTab === 'example' && (
          <>
            <ExampleCardSkeleton />
          </>
        )}
        {activeTab === 'template' && (
          <>
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
          </>
        )}
      </div>
    </div>
  );
}
