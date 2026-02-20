"use client";

import { Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import ExampleGallery from "@/components/ExampleGallery";
import DependencyGraph from "@/components/DependencyGraph";
import {
  ArrowLeft,
  CheckCircle2,
  Clock,
  Globe,
  Github,
  Tag,
} from "lucide-react";
import Link from "next/link";
import { useParams } from "next/navigation";

function ContractDetailsContent() {
  const params = useParams();
  const id = params.id as string;

  const {
    data: contract,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["contract", id],
    queryFn: () => api.getContract(id),
  });

  const { data: dependencies, isLoading: depsLoading } = useQuery({
    queryKey: ["contract-dependencies", id],
    queryFn: () => api.getContractDependencies(id),
    enabled: !!contract,
  });

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="animate-pulse space-y-8">
          <div className="h-8 bg-gray-200 dark:bg-gray-800 rounded w-1/3" />
          <div className="h-4 bg-gray-200 dark:bg-gray-800 rounded w-1/2" />
          <div className="h-64 bg-gray-200 dark:bg-gray-800 rounded-xl" />
        </div>
      </div>
    );
  }

  if (error || !contract) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="p-4 bg-red-50 text-red-600 rounded-lg">
          Failed to load contract details
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-500">
      <Link
        href="/contracts"
        className="inline-flex items-center gap-2 text-gray-500 hover:text-gray-900 dark:hover:text-white mb-8 transition-colors"
      >
        <ArrowLeft className="w-4 h-4" />
        Back to contracts
      </Link>

      {/* Header */}
      <div className="mb-12">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-2">
              {contract.name}
            </h1>
            <div className="flex items-center gap-3 text-gray-500 dark:text-gray-400">
              <span className="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-sm">
                {contract.contract_id}
              </span>
              {contract.is_verified && (
                <span className="flex items-center gap-1 text-green-600 dark:text-green-400 text-sm font-medium">
                  <CheckCircle2 className="w-4 h-4" />
                  Verified
                </span>
              )}
            </div>
          </div>

          <div className="flex gap-2">
            {/* Publisher actions/links could go here */}
          </div>
        </div>

        {contract.description && (
          <p className="text-xl text-gray-600 dark:text-gray-300 max-w-3xl mb-6">
            {contract.description}
          </p>
        )}

        <div className="flex flex-wrap gap-2">
          {contract.tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-sm font-medium"
            >
              <Tag className="w-3 h-3" />
              {tag}
            </span>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-12">
          {/* Dependency Graph */}
          <section>
            <DependencyGraph
              dependencies={dependencies || []}
              isLoading={depsLoading}
            />
          </section>

          {/* Examples Gallery */}
          <section>
            <ExampleGallery contractId={contract.id} />
          </section>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-4">
              Contract Details
            </h3>

            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500 dark:text-gray-400">Network</dt>
                <dd className="font-medium text-gray-900 dark:text-white capitalize">
                  {contract.network}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500 dark:text-gray-400">Published</dt>
                <dd className="font-medium text-gray-900 dark:text-white">
                  {new Date(contract.created_at).toLocaleDateString()}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500 dark:text-gray-400">
                  Last Updated
                </dt>
                <dd className="font-medium text-gray-900 dark:text-white">
                  {new Date(contract.updated_at).toLocaleDateString()}
                </dd>
              </div>
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function ContractPage() {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-950">
      <Suspense fallback={null}>
        <ContractDetailsContent />
      </Suspense>
    </div>
  );
}
