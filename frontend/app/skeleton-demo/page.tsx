'use client';

import { useState } from 'react';
import ContractCardSkeleton from '@/components/ContractCardSkeleton';
import ExampleCardSkeleton from '@/components/ExampleCardSkeleton';
import TemplateCardSkeleton from '@/components/TemplateCardSkeleton';
import Navbar from '@/components/Navbar';

export default function SkeletonDemoPage() {
  const [activeTab, setActiveTab] = useState<'contract' | 'example' | 'template'>('contract');

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-950">
      <Navbar />
      
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2">Skeleton Loader Demo</h1>
          <p className="text-gray-600 dark:text-gray-400 mb-6">
            Preview of loading states for different card types
          </p>

          <div className="flex gap-2 mb-6 flex-wrap">
            <button
              onClick={() => setActiveTab('contract')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'contract'
                  ? 'bg-blue-600 text-white'
                  : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700'
              }`}
            >
              Contract Card
            </button>
            <button
              onClick={() => setActiveTab('example')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'example'
                  ? 'bg-blue-600 text-white'
                  : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700'
              }`}
            >
              Example Card
            </button>
            <button
              onClick={() => setActiveTab('template')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'template'
                  ? 'bg-blue-600 text-white'
                  : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700'
              }`}
            >
              Template Card
            </button>
          </div>

          <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-6">
            <p className="text-sm text-blue-800 dark:text-blue-200">
              💡 <strong>Tip:</strong> These skeletons appear automatically when data is loading. 
              The pulse animation provides visual feedback to users.
            </p>
          </div>
        </div>

        {activeTab === 'contract' && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <ContractCardSkeleton />
            <ContractCardSkeleton />
            <ContractCardSkeleton />
            <ContractCardSkeleton />
            <ContractCardSkeleton />
            <ContractCardSkeleton />
          </div>
        )}

        {activeTab === 'example' && (
          <div className="grid grid-cols-1 gap-8 max-w-4xl">
            <ExampleCardSkeleton />
            <ExampleCardSkeleton />
            <ExampleCardSkeleton />
          </div>
        )}

        {activeTab === 'template' && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
            <TemplateCardSkeleton />
          </div>
        )}
      </div>
    </div>
  );
}
