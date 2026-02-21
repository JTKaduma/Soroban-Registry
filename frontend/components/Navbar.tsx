'use client';

import { Package, GitBranch } from 'lucide-react';
import Link from 'next/link';
import ThemeToggle from './ThemeToggle';

export default function Navbar() {
    return (
        <nav className="border-b border-border bg-background/80 backdrop-blur-sm sticky top-0 z-50">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div className="flex items-center justify-between h-16">
                    <Link href="/" className="flex items-center gap-2">
                        <Package className="w-8 h-8 text-primary" />
                        <span className="text-xl font-bold bg-linear-to-r from-blue-600 to-purple-600 dark:from-blue-400 dark:to-purple-400 bg-clip-text text-transparent">
                            Soroban Registry
                        </span>
                    </Link>
                    <div className="flex items-center gap-4">
                        <Link
                            href="/contracts"
                            className="text-muted-foreground hover:text-foreground transition-colors text-sm font-medium"
                        >
                            Browse
                        </Link>
                        <Link
                            href="/graph"
                            className="flex items-center gap-1.5 text-muted-foreground hover:text-foreground transition-colors text-sm font-medium"
                        >
                            <GitBranch className="w-4 h-4" />
                            Graph
                        </Link>
                        <Link
                            href="/publish"
                            className="px-4 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 transition-opacity font-medium text-sm"
                        >
                            Publish Contract
                        </Link>
                        <div className="border-l border-border pl-4 ml-2">
                            <ThemeToggle />
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    );
}
