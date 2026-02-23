interface ResultsCountProps {
  visibleCount: number;
  totalCount: number;
}

export function ResultsCount({ visibleCount, totalCount }: ResultsCountProps) {
  return (
    <div className="text-sm text-gray-600 dark:text-gray-400">
      Showing {visibleCount} of {totalCount} contracts
    </div>
  );
}
