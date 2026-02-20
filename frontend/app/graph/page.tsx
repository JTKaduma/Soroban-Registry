import { Suspense } from 'react';
import { Package, GitBranch } from 'lucide-react';
import { GraphContent } from './graph-content';
import Link from 'next/link';

export const dynamic = 'force-dynamic';

export const metadata = {
    title: 'Dependency Graph — Soroban Registry',
    description: 'Interactive visualization of contract dependencies in the Soroban smart contract ecosystem.',
};

export default function GraphPage() {
    return (
        <div className="min-h-screen bg-gray-950">
            {/* Navigation */}
            <nav className="border-b border-gray-800 bg-gray-900/80 backdrop-blur-sm sticky top-0 z-50">
                <div className="max-w-[100vw] mx-auto px-4 sm:px-6 lg:px-8">
                    <div className="flex items-center justify-between h-16">
                        <Link href="/" className="flex items-center gap-2">
                            <Package className="w-8 h-8 text-blue-600" />
                            <span className="text-xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
                                Soroban Registry
                            </span>
                        </Link>
                        <div className="flex items-center gap-4">
                            <Link
                                href="/contracts"
                                className="text-gray-300 hover:text-blue-400 transition-colors"
                            >
                                Browse
                            </Link>
                            <Link
                                href="/graph"
                                className="flex items-center gap-1.5 text-blue-400 font-medium"
                            >
                                <GitBranch className="w-4 h-4" />
                                Graph
                            </Link>
                            <Link
                                href="/publish"
                                className="px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors font-medium"
                            >
                                Publish
                            </Link>
                        </div>
                    </div>
                </div>
            </nav>

            <Suspense
                fallback={
                    <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
                        <div className="text-center">
                            <div className="inline-block w-10 h-10 border-4 border-blue-600 border-t-transparent rounded-full animate-spin mb-4" />
                            <p className="text-gray-400 text-sm">Loading dependency graph…</p>
                        </div>
                    </div>
                }
            >
                <GraphContent />
            </Suspense>
        </div>
    );
}
