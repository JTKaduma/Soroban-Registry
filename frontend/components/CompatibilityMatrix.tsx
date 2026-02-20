'use client';

import { CompatibilityMatrix, CompatibilityEntry, api } from '@/lib/api';
import { AlertTriangle, CheckCircle, XCircle, Download, FileJson } from 'lucide-react';

interface CompatibilityMatrixDisplayProps {
    data: CompatibilityMatrix;
    contractId: string;
}

function CompatibilityBadge({ isCompatible }: { isCompatible: boolean }) {
    if (isCompatible) {
        return (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300">
                <CheckCircle className="w-3 h-3" />
                Compatible
            </span>
        );
    }
    return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300">
            <XCircle className="w-3 h-3" />
            Incompatible
        </span>
    );
}

export function CompatibilityMatrixDisplay({ data, contractId }: CompatibilityMatrixDisplayProps) {
    const sourceVersions = Object.keys(data.versions).sort();
    const csvUrl = api.getCompatibilityExportUrl(contractId, 'csv');
    const jsonUrl = api.getCompatibilityExportUrl(contractId, 'json');

    // Collect all unique target contracts across all source versions
    const allEntries: CompatibilityEntry[] = Object.values(data.versions).flat();
    const uniqueTargets = Array.from(
        new Map(
            allEntries.map((e) => [
                e.target_contract_id + '@' + e.target_version,
                e,
            ])
        ).values()
    ).sort((a, b) =>
        `${a.target_contract_name}@${a.target_version}`.localeCompare(
            `${b.target_contract_name}@${b.target_version}`
        )
    );

    return (
        <div className="space-y-6">
            {/* Warnings */}
            {data.warnings.length > 0 && (
                <div className="rounded-xl border border-amber-300 bg-amber-50 dark:bg-amber-900/20 dark:border-amber-700 p-4">
                    <div className="flex items-center gap-2 mb-2">
                        <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 flex-shrink-0" />
                        <span className="font-semibold text-amber-800 dark:text-amber-300 text-sm">
                            Incompatibility Warning{data.warnings.length > 1 ? 's' : ''}
                        </span>
                    </div>
                    <ul className="space-y-1 ml-7">
                        {data.warnings.map((w, i) => (
                            <li key={i} className="text-sm text-amber-700 dark:text-amber-300">
                                {w}
                            </li>
                        ))}
                    </ul>
                </div>
            )}

            {/* Summary + Export row */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                <p className="text-sm text-gray-500 dark:text-gray-400">
                    {data.total_entries} compatibility{' '}
                    {data.total_entries === 1 ? 'record' : 'records'} across{' '}
                    {sourceVersions.length} source{' '}
                    {sourceVersions.length === 1 ? 'version' : 'versions'}
                </p>
                <div className="flex gap-2">
                    <a
                        href={csvUrl}
                        download="compatibility.csv"
                        className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-750 transition-colors"
                    >
                        <Download className="w-3.5 h-3.5" />
                        CSV
                    </a>
                    <a
                        href={jsonUrl}
                        download="compatibility.json"
                        className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-750 transition-colors"
                    >
                        <FileJson className="w-3.5 h-3.5" />
                        JSON
                    </a>
                </div>
            </div>

            {/* Matrix table */}
            {sourceVersions.length === 0 || uniqueTargets.length === 0 ? (
                <div className="text-center py-12 rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900">
                    <CheckCircle className="w-10 h-10 text-gray-300 dark:text-gray-600 mx-auto mb-3" />
                    <p className="text-gray-500 dark:text-gray-400 text-sm">
                        No compatibility data yet. Add entries using the API.
                    </p>
                </div>
            ) : (
                <div className="overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
                    <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700 text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-800/60">
                            <tr>
                                <th className="sticky left-0 z-10 bg-gray-50 dark:bg-gray-800/60 px-4 py-3 text-left font-semibold text-gray-700 dark:text-gray-300 whitespace-nowrap border-r border-gray-200 dark:border-gray-700">
                                    Target Contract @ Version
                                </th>
                                {sourceVersions.map((v) => (
                                    <th
                                        key={v}
                                        className="px-4 py-3 text-center font-semibold text-gray-700 dark:text-gray-300 whitespace-nowrap"
                                    >
                                        <span className="inline-block px-2 py-0.5 rounded-md bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 font-mono text-xs">
                                            v{v}
                                        </span>
                                    </th>
                                ))}
                            </tr>
                        </thead>
                        <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-100 dark:divide-gray-800">
                            {uniqueTargets.map((target) => (
                                <tr
                                    key={`${target.target_contract_id}@${target.target_version}`}
                                    className="hover:bg-gray-50 dark:hover:bg-gray-800/40 transition-colors"
                                >
                                    <td className="sticky left-0 z-10 bg-white dark:bg-gray-900 px-4 py-3 font-medium text-gray-900 dark:text-white border-r border-gray-100 dark:border-gray-800 whitespace-nowrap">
                                        <div>{target.target_contract_name}</div>
                                        <div className="text-xs text-gray-400 font-mono mt-0.5">
                                            v{target.target_version}
                                            {target.stellar_version && (
                                                <span className="ml-2 text-gray-400">
                                                    (Stellar {target.stellar_version})
                                                </span>
                                            )}
                                        </div>
                                    </td>
                                    {sourceVersions.map((sv) => {
                                        const entries = data.versions[sv] || [];
                                        const match = entries.find(
                                            (e) =>
                                                e.target_contract_id === target.target_contract_id &&
                                                e.target_version === target.target_version
                                        );
                                        return (
                                            <td
                                                key={sv}
                                                className={`px-4 py-3 text-center ${match
                                                        ? match.is_compatible
                                                            ? 'bg-green-50/40 dark:bg-green-900/10'
                                                            : 'bg-red-50/40 dark:bg-red-900/10'
                                                        : ''
                                                    }`}
                                            >
                                                {match ? (
                                                    <div className="flex justify-center">
                                                        <CompatibilityBadge isCompatible={match.is_compatible} />
                                                    </div>
                                                ) : (
                                                    <span className="text-gray-300 dark:text-gray-600">—</span>
                                                )}
                                            </td>
                                        );
                                    })}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
}
