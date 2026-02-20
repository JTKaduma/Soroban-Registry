'use client';

import {
    Search, ZoomIn, ZoomOut, Maximize2, Download, FileImage,
    GitBranch, Circle, Activity, Sparkles, ChevronUp, ChevronDown
} from 'lucide-react';

interface GraphControlsProps {
    searchQuery: string;
    onSearchChange: (q: string) => void;
    networkFilter: string;
    onNetworkFilterChange: (n: string) => void;
    demoMode: boolean;
    onDemoModeChange: (v: boolean) => void;
    demoNodeCount: number;
    onDemoNodeCountChange: (v: number) => void;
    totalNodes: number;
    totalEdges: number;
    criticalCount: number;
    searchMatchCount: number;
    searchMatchIndex: number;
    onPrevMatch: () => void;
    onNextMatch: () => void;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onResetZoom: () => void;
    onExportSVG: () => void;
    onExportPNG: () => void;
}

export default function GraphControls({
    searchQuery,
    onSearchChange,
    networkFilter,
    onNetworkFilterChange,
    demoMode,
    onDemoModeChange,
    demoNodeCount,
    onDemoNodeCountChange,
    totalNodes,
    totalEdges,
    criticalCount,
    searchMatchCount,
    searchMatchIndex,
    onPrevMatch,
    onNextMatch,
    onZoomIn,
    onZoomOut,
    onResetZoom,
    onExportSVG,
    onExportPNG,
}: GraphControlsProps) {
    return (
        <>
            {/* Top-left: Search + Filters */}
            <div className="absolute top-4 left-4 z-30 flex flex-col gap-3 max-w-xs">
                {/* Search */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden">
                    <div className="relative flex items-center">
                        <Search className="absolute left-3 w-4 h-4 text-gray-500" />
                        <input
                            id="graph-search"
                            type="text"
                            value={searchQuery}
                            onChange={(e) => onSearchChange(e.target.value)}
                            onKeyDown={(e) => { if (e.key === 'Enter' && searchMatchCount > 0) onNextMatch(); }}
                            placeholder="Search contracts…"
                            className="w-full pl-9 pr-3 py-2.5 bg-transparent text-sm text-white placeholder-gray-500 focus:outline-none"
                        />
                        {searchQuery && searchMatchCount > 0 && (
                            <div className="flex items-center gap-0.5 pr-1.5 shrink-0">
                                <span className="text-xs text-gray-400 tabular-nums px-1">
                                    {searchMatchIndex + 1}/{searchMatchCount}
                                </span>
                                <button
                                    onClick={onPrevMatch}
                                    className="p-0.5 text-gray-400 hover:text-white transition-colors rounded"
                                    title="Previous match"
                                >
                                    <ChevronUp className="w-3.5 h-3.5" />
                                </button>
                                <button
                                    onClick={onNextMatch}
                                    className="p-0.5 text-gray-400 hover:text-white transition-colors rounded"
                                    title="Next match"
                                >
                                    <ChevronDown className="w-3.5 h-3.5" />
                                </button>
                            </div>
                        )}
                        {searchQuery && searchMatchCount === 0 && (
                            <span className="text-xs text-red-400 pr-2.5 shrink-0">No results</span>
                        )}
                    </div>
                </div>

                {/* Filters */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl space-y-3">
                    <div>
                        <label className="text-xs text-gray-400 mb-1 block">Network</label>
                        <select
                            id="graph-network-filter"
                            value={networkFilter}
                            onChange={(e) => onNetworkFilterChange(e.target.value)}
                            className="w-full px-3 py-1.5 rounded-lg bg-gray-800 border border-gray-700 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-blue-500"
                        >
                            <option value="">All Networks</option>
                            <option value="mainnet">Mainnet</option>
                            <option value="testnet">Testnet</option>
                            <option value="futurenet">Futurenet</option>
                        </select>
                    </div>

                    {/* Demo Mode */}
                    <div className="border-t border-gray-800 pt-3">
                        <label className="flex items-center gap-2 cursor-pointer">
                            <input
                                id="graph-demo-toggle"
                                type="checkbox"
                                checked={demoMode}
                                onChange={(e) => onDemoModeChange(e.target.checked)}
                                className="rounded border-gray-600 text-blue-600 focus:ring-blue-500 bg-gray-800"
                            />
                            <span className="text-sm text-gray-300 flex items-center gap-1.5">
                                <Sparkles className="w-3.5 h-3.5 text-amber-400" />
                                Demo Mode
                            </span>
                        </label>
                        {demoMode && (
                            <div className="mt-2">
                                <label className="text-xs text-gray-500 mb-1 block">Contracts: {demoNodeCount.toLocaleString()}</label>
                                <input
                                    id="graph-demo-count"
                                    type="range"
                                    min={50}
                                    max={10000}
                                    step={50}
                                    value={demoNodeCount}
                                    onChange={(e) => onDemoNodeCountChange(Number(e.target.value))}
                                    className="w-full h-1.5 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
                                />
                                <div className="flex justify-between text-[10px] text-gray-600 mt-0.5">
                                    <span>50</span>
                                    <span>10,000</span>
                                </div>
                            </div>
                        )}
                    </div>
                </div>

                {/* Legend */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl">
                    <p className="text-xs text-gray-400 mb-2 font-medium">Legend</p>
                    <div className="space-y-1.5 text-xs">
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-green-500 fill-green-500" />
                            <span className="text-gray-300">Mainnet</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-blue-500 fill-blue-500" />
                            <span className="text-gray-300">Testnet</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-purple-500 fill-purple-500" />
                            <span className="text-gray-300">Futurenet</span>
                        </div>
                        <div className="flex items-center gap-2 pt-1 border-t border-gray-800">
                            <div className="w-3 h-3 rounded-full border-2 border-amber-400" />
                            <span className="text-gray-300">Critical (≥5 deps)</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-gray-400 mx-0.5" />
                            <span className="text-gray-400">Larger = more dependents</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Top-right: Stats bar */}
            <div className="absolute top-4 right-4 z-30">
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl flex items-center gap-4">
                    <div className="text-center">
                        <div className="text-lg font-bold text-white">{totalNodes.toLocaleString()}</div>
                        <div className="text-[10px] text-gray-400 uppercase tracking-wider">Nodes</div>
                    </div>
                    <div className="w-px h-8 bg-gray-700" />
                    <div className="text-center">
                        <div className="text-lg font-bold text-white">{totalEdges.toLocaleString()}</div>
                        <div className="text-[10px] text-gray-400 uppercase tracking-wider">Edges</div>
                    </div>
                    <div className="w-px h-8 bg-gray-700" />
                    <div className="text-center">
                        <div className="text-lg font-bold text-amber-400">{criticalCount}</div>
                        <div className="text-[10px] text-gray-400 uppercase tracking-wider">Critical</div>
                    </div>
                </div>
            </div>

            {/* Bottom-right: Zoom + Export controls */}
            <div className="absolute bottom-4 right-4 z-30 flex flex-col gap-2">
                {/* Zoom controls */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden">
                    <button
                        id="graph-zoom-in"
                        onClick={onZoomIn}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors"
                        title="Zoom in"
                    >
                        <ZoomIn className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" />
                    <button
                        id="graph-zoom-out"
                        onClick={onZoomOut}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors"
                        title="Zoom out"
                    >
                        <ZoomOut className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" />
                    <button
                        id="graph-reset-zoom"
                        onClick={onResetZoom}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors"
                        title="Reset zoom"
                    >
                        <Maximize2 className="w-4 h-4" />
                    </button>
                </div>

                {/* Export controls */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden">
                    <button
                        id="graph-export-svg"
                        onClick={onExportSVG}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors"
                        title="Export as SVG"
                    >
                        <Download className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" />
                    <button
                        id="graph-export-png"
                        onClick={onExportPNG}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors"
                        title="Export as PNG"
                    >
                        <FileImage className="w-4 h-4" />
                    </button>
                </div>
            </div>
        </>
    );
}
