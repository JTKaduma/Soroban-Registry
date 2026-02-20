import { Contract } from '@/lib/api';
import { CheckCircle2, Clock, ExternalLink, Tag } from 'lucide-react';
import Link from 'next/link';
import React from 'react';
import HealthWidget from './HealthWidget';

interface ContractCardProps {
  contract: Contract;
}

export default function ContractCard({ contract }: ContractCardProps) {
  const networkColors = {
    mainnet: 'bg-green-500/10 text-green-600 border-green-500/20',
    testnet: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
    futurenet: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  };

  return (
    <Link href={`/contracts/${contract.id}`}>
      <div className="group relative overflow-hidden rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-6 transition-all hover:border-blue-500/50 hover:shadow-lg hover:shadow-blue-500/10">
        {/* Gradient overlay on hover */}
        <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-purple-500/5 opacity-0 transition-opacity group-hover:opacity-100" />

        <div className="relative">
          {/* Header */}
          <div className="flex items-start justify-between mb-3">
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-1">
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                  {contract.name}
                </h3>
                {contract.is_verified && (
                  <CheckCircle2 className="w-5 h-5 text-green-500" />
                )}
              </div>
              <p className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                {contract.contract_id.slice(0, 8)}...{contract.contract_id.slice(-6)}
              </p>
            </div>

            <span className={`px-3 py-1 rounded-full text-xs font-medium border ${networkColors[contract.network]}`}>
              {contract.network}
            </span>
          </div>

          {/* Description */}
          {contract.description && (
            <p className="text-sm text-gray-600 dark:text-gray-300 mb-4 line-clamp-2">
              {contract.description}
            </p>
          )}

          {/* Tags */}
          {contract.tags && contract.tags.length > 0 && (
            <div className="flex flex-wrap gap-2 mb-4">
              {contract.tags.slice(0, 3).map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-gray-100 dark:bg-gray-800 text-xs text-gray-600 dark:text-gray-300"
                >
                  <Tag className="w-3 h-3" />
                  {tag}
                </span>
              ))}
              {contract.tags.length > 3 && (
                <span className="px-2 py-1 text-xs text-gray-500">
                  +{contract.tags.length - 3} more
                </span>
              )}
            </div>
          )}

          {/* Health Widget */}
          <div onClick={(e: React.MouseEvent) => e.preventDefault()}>
            <HealthWidget contract={contract} />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
            <div className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              {new Date(contract.created_at).toLocaleDateString()}
            </div>
            <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <span>View details</span>
              <ExternalLink className="w-3 h-3" />
            </div>
          </div>
        </div>
      </div>
    </Link>
  );
}
