import { Suspense } from 'react';
import { Package, GitBranch } from 'lucide-react';
import { ContractsContent } from './contracts-content';

export const dynamic = 'force-dynamic';

export default function ContractsPage() {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-950">
      {/* Navigation */}
      <nav className="border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <a href="/" className="flex items-center gap-2">
              <Package className="w-8 h-8 text-blue-600" />
              <span className="text-xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
                Soroban Registry
              </span>
            </a>
            <div className="flex items-center gap-4">
              <a
                href="/contracts"
                className="text-blue-600 dark:text-blue-400 font-medium"
              >
                Browse
              </a>
              <a
                href="/graph"
                className="flex items-center gap-1.5 text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
              >
                <GitBranch className="w-4 h-4" />
                Graph
              </a>
              <a
                href="/publish"
                className="px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors font-medium"
              >
                Publish Contract
              </a>
            </div>
          </div>
        </div>
      </nav>

      <Suspense
        fallback={
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
            <div className="text-center py-12">
              <div className="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin" />
            </div>
          </div>
        }
      >
        <ContractsContent />
      </Suspense>
    </div>
  );
}
