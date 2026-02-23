import LoadingSkeleton from './LoadingSkeleton';

export default function ContractCardSkeleton() {
  return (
    <div className="relative overflow-hidden rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-6">
      <div className="relative">
        {/* Header */}
        <div className="flex items-start justify-between mb-3">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <LoadingSkeleton width="60%" height="1.5rem" />
            </div>
            <LoadingSkeleton width="40%" height="0.875rem" className="mt-2" />
          </div>
          <LoadingSkeleton width="5rem" height="1.75rem" className="rounded-full" />
        </div>

        {/* Description */}
        <div className="mb-4 space-y-2">
          <LoadingSkeleton width="100%" height="0.875rem" />
          <LoadingSkeleton width="85%" height="0.875rem" />
        </div>

        {/* Tags */}
        <div className="flex flex-wrap gap-2 mb-4">
          <LoadingSkeleton width="4rem" height="1.75rem" className="rounded-md" />
          <LoadingSkeleton width="5rem" height="1.75rem" className="rounded-md" />
          <LoadingSkeleton width="4.5rem" height="1.75rem" className="rounded-md" />
        </div>

        {/* Health Widget */}
        <div className="mb-4">
          <LoadingSkeleton width="100%" height="3rem" className="rounded-lg" />
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between">
          <LoadingSkeleton width="6rem" height="0.75rem" />
          <LoadingSkeleton width="5rem" height="0.75rem" />
        </div>
      </div>
    </div>
  );
}
