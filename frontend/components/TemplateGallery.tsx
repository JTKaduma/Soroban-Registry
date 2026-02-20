'use client';

import { useState } from 'react';
import TemplateCard from './TemplateCard';
import { Template } from '@/lib/api';
import { LayoutGrid } from 'lucide-react';

const CATEGORIES = ['all', 'token', 'dex', 'bridge', 'oracle', 'lending'];

export default function TemplateGallery({ templates }: { templates: Template[] }) {
    const [activeCategory, setActiveCategory] = useState('all');

    const filtered = activeCategory === 'all'
        ? templates
        : templates.filter((t) => t.category === activeCategory);

    return (
        <div>
            <div className="flex flex-wrap gap-2 mb-8">
                {CATEGORIES.map((cat) => (
                    <button
                        key={cat}
                        onClick={() => setActiveCategory(cat)}
                        className={`px-4 py-2 rounded-full text-sm font-medium border transition-all ${activeCategory === cat
                            ? 'bg-blue-600 text-white border-blue-600 shadow-md shadow-blue-500/20'
                            : 'bg-white dark:bg-gray-900 text-gray-600 dark:text-gray-300 border-gray-200 dark:border-gray-700 hover:border-blue-400'
                            }`}
                    >
                        {cat.charAt(0).toUpperCase() + cat.slice(1)}
                    </button>
                ))}
            </div>

            {filtered.length === 0 ? (
                <div className="text-center py-16 bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800">
                    <LayoutGrid className="w-12 h-12 text-gray-400 mx-auto mb-4" />
                    <p className="text-gray-600 dark:text-gray-400">No templates in this category yet.</p>
                </div>
            ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {filtered.map((t) => (
                        <TemplateCard key={t.id} template={t} />
                    ))}
                </div>
            )}
        </div>
    );
}
