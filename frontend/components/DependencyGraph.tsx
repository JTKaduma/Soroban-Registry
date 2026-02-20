"use client";

import { DependencyTreeNode } from "@/lib/api";
import {
  ChevronRight,
  ChevronDown,
  Box,
  ShieldCheck,
  AlertCircle,
} from "lucide-react";
import { useState } from "react";

interface DependencyNodeProps {
  node: DependencyTreeNode;
  depth?: number;
  isLast?: boolean;
}

function DependencyNode({
  node,
  depth = 0,
  isLast = false,
}: DependencyNodeProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const hasChildren = node.dependencies && node.dependencies.length > 0;

  // Indentation logic
  const indentation = depth * 24;

  return (
    <div className="select-none">
      <div
        className={`
          flex items-center gap-2 py-2 px-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800/50 
          transition-colors border-l-2
          ${depth === 0 ? "border-primary-500 bg-primary-50/10" : "border-transparent"}
        `}
        style={{ marginLeft: `${indentation}px` }}
      >
        <button
          onClick={() => hasChildren && setIsExpanded(!isExpanded)}
          className={`
            p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors
            ${!hasChildren ? "invisible" : ""}
          `}
        >
          {isExpanded ? (
            <ChevronDown className="w-4 h-4 text-gray-500" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-500" />
          )}
        </button>

        <div className="flex items-center gap-2">
          <Box className="w-4 h-4 text-blue-500" />
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {node.name}
          </span>
        </div>

        <div className="flex items-center gap-2 ml-2">
          <span className="text-xs font-mono px-2 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700">
            {node.constraint_to_parent}
          </span>

          {node.contract_id === "unknown" ? (
            <span
              className="flex items-center gap-1 text-xs text-red-500"
              title="Contract not found in registry"
            >
              <AlertCircle className="w-3 h-3" />
              Unresolved
            </span>
          ) : (
            <span
              className="flex items-center gap-1 text-xs text-green-600 dark:text-green-500"
              title="Resolved"
            >
              <ShieldCheck className="w-3 h-3" />
            </span>
          )}
        </div>
      </div>

      {isExpanded && hasChildren && (
        <div className="relative">
          {/* Vertical connector line could go here */}
          {node.dependencies.map((child, index) => (
            <DependencyNode
              key={`${child.contract_id}-${index}`}
              node={child}
              depth={depth + 1}
              isLast={index === node.dependencies.length - 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface DependencyGraphProps {
  dependencies: DependencyTreeNode[];
  isLoading?: boolean;
}

export default function DependencyGraph({
  dependencies,
  isLoading,
}: DependencyGraphProps) {
  if (isLoading) {
    return (
      <div className="space-y-2 animate-pulse">
        <div className="h-10 bg-gray-100 dark:bg-gray-800 rounded w-full" />
        <div className="h-10 bg-gray-100 dark:bg-gray-800 rounded w-3/4 ml-6" />
        <div className="h-10 bg-gray-100 dark:bg-gray-800 rounded w-1/2 ml-12" />
      </div>
    );
  }

  if (!dependencies || dependencies.length === 0) {
    return (
      <div className="text-center py-8 text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-900/50 rounded-xl border border-dashed border-gray-200 dark:border-gray-800">
        No dependencies declared
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 overflow-hidden">
      <div className="p-4 border-b border-gray-200 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-900/50">
        <h3 className="font-semibold text-gray-900 dark:text-white flex items-center gap-2">
          <Box className="w-4 h-4" />
          Dependency Graph
        </h3>
      </div>
      <div className="p-4 overflow-x-auto">
        {dependencies.map((rootNode, index) => (
          <DependencyNode
            key={`${rootNode.contract_id}-${index}`}
            node={rootNode}
            isLast={index === dependencies.length - 1}
          />
        ))}
      </div>
    </div>
  );
}
