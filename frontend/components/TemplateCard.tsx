import Link from 'next/link';
import { Download, Tag } from 'lucide-react';
import { Template } from '@/lib/api';

const CATEGORY_COLORS: Record<string, string> = {
    token: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
    dex: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
    bridge: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
    oracle: 'bg-green-500/10 text-green-600 border-green-500/20',
    lending: 'bg-pink-500/10 text-pink-600 border-pink-500/20',
};

export default function TemplateCard({ template }: { template: Template }) {
    const colorClass = CATEGORY_COLORS[template.category] ?? 'bg-gray-500/10 text-gray-600 border-gray-500/20';

    return (
        <Link href={`/templates/${template.slug}`}>
            <div className="group relative overflow-hidden rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-6 transition-all hover:border-blue-500/50 hover:shadow-lg hover:shadow-blue-500/10 cursor-pointer">
                <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-purple-500/5 opacity-0 transition-opacity group-hover:opacity-100" />

                <div className="relative">
                    <div className="flex items-start justify-between mb-3">
                        <div className="flex-1">
                            <div className="flex items-center gap-2 mb-1">
                                <h3 className="text-lg font-semibold text-gray-900 dark:text-white group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                                    {template.name}
                                </h3>
                                <span className="text-xs text-gray-400 font-mono">v{template.version}</span>
                            </div>
                            <span className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium border ${colorClass}`}>
                                {template.category}
                            </span>
                        </div>
                        <div className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400 ml-2">
                            <Download className="w-4 h-4" />
                            <span>{template.install_count.toLocaleString()}</span>
                        </div>
                    </div>

                    {template.description && (
                        <p className="text-sm text-gray-600 dark:text-gray-300 mb-4 line-clamp-2">{template.description}</p>
                    )}

                    {template.parameters.length > 0 && (
                        <div className="flex flex-wrap gap-2 mb-4">
                            {template.parameters.slice(0, 3).map((p) => (
                                <span key={p.name} className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-gray-100 dark:bg-gray-800 text-xs text-gray-600 dark:text-gray-300">
                                    <Tag className="w-3 h-3" />
                                    {p.name}
                                </span>
                            ))}
                            {template.parameters.length > 3 && (
                                <span className="px-2 py-1 text-xs text-gray-500">+{template.parameters.length - 3} more</span>
                            )}
                        </div>
                    )}

                    <div className="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
                        <code className="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">
                            soroban-registry template clone {template.slug} my-contract
                        </code>
                    </div>
                </div>
            </div>
        </Link>
    );
}
