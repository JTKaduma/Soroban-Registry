"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { Shield, ShieldAlert, ShieldCheck, ShieldQuestion, ExternalLink, ChevronDown, ChevronUp } from "lucide-react";
import { useState } from "react";

export default function FormalVerificationPanel({ contractId }: { contractId: string }) {
    const [expandedProperty, setExpandedProperty] = useState<string | null>(null);

    const { data: reports, isLoading, error } = useQuery({
        queryKey: ["formal-verification", contractId],
        queryFn: () => api.getFormalVerificationResults(contractId),
    });

    if (isLoading) {
        return (
            <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 animate-pulse">
                <div className="h-6 bg-gray-200 dark:bg-gray-800 rounded w-1/2 mb-4"></div>
                <div className="space-y-3">
                    <div className="h-4 bg-gray-200 dark:bg-gray-800 rounded"></div>
                    <div className="h-4 bg-gray-200 dark:bg-gray-800 rounded w-5/6"></div>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 flex items-center justify-between text-gray-500">
                <div className="flex items-center gap-3">
                    <ShieldAlert className="w-5 h-5 text-gray-400" />
                    <span className="text-sm font-medium">Verification checks unavailable</span>
                </div>
            </div>
        );
    }

    const latestReport = reports && reports.length > 0 ? reports[0] : null;

    if (!latestReport) {
        return (
            <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6">
                <div className="flex items-center gap-3 mb-2">
                    <Shield className="w-5 h-5 text-gray-400" />
                    <h3 className="font-semibold text-gray-900 dark:text-white">Formal Verification</h3>
                </div>
                <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                    No formal verification properties have been analyzed for this contract yet.
                </p>
                <a
                    href="https://soroban.stellar.org/docs/fundamentals-and-concepts/security"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-sm font-medium text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
                >
                    Learn about writing safer contracts <ExternalLink className="w-3 h-3" />
                </a>
            </div>
        );
    }

    const provedCount = latestReport.properties.filter((p) => p.result.status === "Proved").length;
    const violatedCount = latestReport.properties.filter((p) => p.result.status === "Violated").length;

    return (
        <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 group">
            <div className="flex flex-col mb-4">
                <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                        <Shield className="w-5 h-5 text-gray-900 dark:text-gray-100" />
                        <h3 className="font-semibold text-gray-900 dark:text-white">Formal Verification</h3>
                    </div>
                    {violatedCount > 0 ? (
                        <span className="flex items-center gap-1 text-xs font-semibold px-2 py-1 bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 rounded-full">
                            <ShieldAlert className="w-3 h-3" /> Issues Found
                        </span>
                    ) : (
                        <span className="flex items-center gap-1 text-xs font-semibold px-2 py-1 bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-full">
                            <ShieldCheck className="w-3 h-3" /> Fully Verified
                        </span>
                    )}
                </div>
                <p className="text-sm text-gray-500 dark:text-gray-400">
                    Analyzed against {latestReport.properties.length} structural invariants.
                    <span className="ml-1 text-gray-700 dark:text-gray-300 font-medium">{provedCount} proved</span>,
                    <span className={violatedCount > 0 ? "ml-1 text-red-600 dark:text-red-400 font-medium" : "ml-1"}> {violatedCount} violations</span>.
                </p>
            </div>

            <div className="space-y-3 mt-6">
                {latestReport.properties.map((propResult) => {
                    const isProved = propResult.result.status === "Proved";
                    const isViolated = propResult.result.status === "Violated";
                    const isUnknown = propResult.result.status === "Unknown";
                    const isExpanded = expandedProperty === propResult.result.id;

                    return (
                        <div
                            key={propResult.result.id}
                            className={`border rounded-lg overflow-hidden transition-colors ${isViolated
                                    ? "border-red-200 dark:border-red-900/30 bg-red-50/50 dark:bg-red-900/10"
                                    : "border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/30"
                                }`}
                        >
                            <button
                                onClick={() => setExpandedProperty(isExpanded ? null : propResult.result.id)}
                                className="w-full flex items-start justify-between p-3 text-left focus:outline-none"
                            >
                                <div className="flex items-start gap-3">
                                    <div className="mt-0.5 flex-shrink-0">
                                        {isProved && <ShieldCheck className="w-4 h-4 text-green-500" />}
                                        {isViolated && <ShieldAlert className="w-4 h-4 text-red-500" />}
                                        {isUnknown && <ShieldQuestion className="w-4 h-4 text-yellow-500" />}
                                    </div>
                                    <div>
                                        <div className="text-sm font-medium text-gray-900 dark:text-white">
                                            {propResult.property.description || propResult.property.invariant}
                                        </div>
                                        <div className="text-xs text-gray-500 mt-1 font-mono uppercase">
                                            {propResult.property.property_id} • {propResult.result.status}
                                        </div>
                                    </div>
                                </div>
                                {isViolated && (
                                    <div className="flex-shrink-0 text-gray-400 ml-2">
                                        {isExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                                    </div>
                                )}
                            </button>

                            {isExpanded && isViolated && propResult.result.counterexample && (
                                <div className="p-3 pt-0 text-sm border-t border-red-100 dark:border-red-900/30 mt-2 bg-red-50 dark:bg-red-900/20">
                                    <div className="font-semibold text-red-800 dark:text-red-300 mb-1 text-xs uppercase tracking-wider">
                                        Counterexample
                                    </div>
                                    <div className="text-red-700 dark:text-red-200">
                                        {propResult.result.counterexample}
                                    </div>
                                </div>
                            )}
                        </div>
                    );
                })}
            </div>

            <div className="mt-5 pt-4 border-t border-gray-100 dark:border-gray-800">
                <div className="text-xs text-gray-400 flex justify-between">
                    <span>Engine v{latestReport.session.verifier_version}</span>
                    <span>{new Date(latestReport.session.created_at).toLocaleDateString()}</span>
                </div>
            </div>
        </div>
    );
}
