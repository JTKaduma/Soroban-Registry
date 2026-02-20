'use client';

import { useQuery } from '@tanstack/react-query';
import { api, GraphNode, GraphEdge } from '@/lib/api';
import DependencyGraph from '@/components/DependencyGraph';
import GraphControls from '@/components/GraphControls';
import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { AlertCircle, Sparkles } from 'lucide-react';

// Generate synthetic demo data for testing at scale
function generateDemoData(nodeCount: number): { nodes: GraphNode[]; edges: GraphEdge[] } {
    const networks: ('mainnet' | 'testnet' | 'futurenet')[] = ['mainnet', 'testnet', 'futurenet'];
    const categories = ['DeFi', 'NFT', 'DAO', 'Oracle', 'Bridge', 'DEX', 'Lending', 'Staking', 'Wallet', 'Token'];
    const tagOptions = ['soroban', 'stellar', 'defi', 'amm', 'lending', 'governance', 'token', 'nft', 'oracle', 'bridge'];
    const nameAdjectives = ['Swift', 'Quantum', 'Solar', 'Stellar', 'Bright', 'Nova', 'Cosmic', 'Nebula', 'Astral', 'Lunar'];
    const nameNouns = ['Swap', 'Vault', 'Pool', 'Bridge', 'Oracle', 'Token', 'Lend', 'Stake', 'DAO', 'Mint'];

    const nodes: GraphNode[] = [];
    for (let i = 0; i < nodeCount; i++) {
        const adj = nameAdjectives[Math.floor(Math.random() * nameAdjectives.length)];
        const noun = nameNouns[Math.floor(Math.random() * nameNouns.length)];
        const tagCount = 1 + Math.floor(Math.random() * 3);
        const tags: string[] = [];
        for (let t = 0; t < tagCount; t++) {
            const tag = tagOptions[Math.floor(Math.random() * tagOptions.length)];
            if (!tags.includes(tag)) tags.push(tag);
        }
        nodes.push({
            id: `demo-${i}`,
            contract_id: `C${Array.from({ length: 55 }, () => 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'[Math.floor(Math.random() * 32)]).join('')}`,
            name: `${adj}${noun}${i > 0 ? i : ''}`,
            network: networks[Math.floor(Math.random() * networks.length)],
            is_verified: Math.random() > 0.6,
            category: categories[Math.floor(Math.random() * categories.length)],
            tags,
        });
    }

    // Create edges — power-law distribution: some nodes get many dependents
    const edges: GraphEdge[] = [];
    const edgeCount = Math.min(nodeCount * 2, nodeCount * (nodeCount - 1) / 2);
    const edgeSet = new Set<string>();
    for (let i = 0; i < edgeCount; i++) {
        // Bias towards lower-index nodes as targets to create hub nodes
        const sourceIdx = Math.floor(Math.random() * nodeCount);
        const targetIdx = Math.floor(Math.pow(Math.random(), 2) * nodeCount);
        if (sourceIdx === targetIdx) continue;
        const key = `${sourceIdx}-${targetIdx}`;
        if (edgeSet.has(key)) continue;
        edgeSet.add(key);
        edges.push({
            source: nodes[sourceIdx].id,
            target: nodes[targetIdx].id,
            dependency_type: Math.random() > 0.7 ? 'imports' : 'calls',
        });
    }

    return { nodes, edges };
}

export function GraphContent() {
    const [networkFilter, setNetworkFilter] = useState<string>('');
    const [searchQuery, setSearchQuery] = useState('');
    const [demoMode, setDemoMode] = useState(false);
    const [demoNodeCount, setDemoNodeCount] = useState(200);
    const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
    const [searchMatchIndex, setSearchMatchIndex] = useState(0);
    const graphRef = useRef<any>(null);

    const { data: apiData, isLoading, error } = useQuery({
        queryKey: ['contract-graph', networkFilter],
        queryFn: () => api.getContractGraph(networkFilter || undefined),
        enabled: !demoMode,
    });

    const demoData = useMemo(
        () => (demoMode ? generateDemoData(demoNodeCount) : null),
        [demoMode, demoNodeCount]
    );

    // Apply client-side network filtering for demo mode
    const filteredDemoData = useMemo(() => {
        if (!demoData || !networkFilter) return demoData;
        const filteredNodes = demoData.nodes.filter((n) => n.network === networkFilter);
        const nodeIds = new Set(filteredNodes.map((n) => n.id));
        const filteredEdges = demoData.edges.filter(
            (e) => nodeIds.has(e.source) && nodeIds.has(e.target)
        );
        return { nodes: filteredNodes, edges: filteredEdges };
    }, [demoData, networkFilter]);

    const graphData = demoMode ? filteredDemoData : apiData;

    // Compute dependency counts for critical node detection
    const dependentCounts = useMemo(() => {
        if (!graphData) return new Map<string, number>();
        const counts = new Map<string, number>();
        for (const edge of graphData.edges) {
            counts.set(edge.target, (counts.get(edge.target) || 0) + 1);
        }
        return counts;
    }, [graphData]);

    const criticalCount = useMemo(() => {
        let count = 0;
        dependentCounts.forEach((v) => { if (v >= 5) count++; });
        return count;
    }, [dependentCounts]);

    const handleNodeClick = useCallback((node: GraphNode | null) => {
        setSelectedNode(node);
    }, []);

    // Search match navigation
    const searchMatches = useMemo(() => {
        if (!searchQuery || !graphData) return [];
        const q = searchQuery.toLowerCase();
        return graphData.nodes
            .filter((n) => n.name.toLowerCase().includes(q) || n.contract_id.toLowerCase().includes(q))
            .map((n) => n.id);
    }, [searchQuery, graphData]);

    // Reset match index when query or matches change
    useEffect(() => {
        setSearchMatchIndex(0);
    }, [searchQuery]);

    // Auto-focus on the active match
    useEffect(() => {
        if (searchMatches.length > 0 && graphRef.current) {
            graphRef.current.focusOnNode(searchMatches[searchMatchIndex] || searchMatches[0]);
        }
    }, [searchMatches, searchMatchIndex]);

    const handlePrevMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i - 1 + searchMatches.length) % searchMatches.length);
    }, [searchMatches.length]);

    const handleNextMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i + 1) % searchMatches.length);
    }, [searchMatches.length]);

    if (!demoMode && isLoading) {
        return (
            <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
                <div className="text-center">
                    <div className="inline-block w-10 h-10 border-4 border-blue-600 border-t-transparent rounded-full animate-spin mb-4" />
                    <p className="text-gray-400 text-sm">Loading graph data…</p>
                </div>
            </div>
        );
    }

    if (!demoMode && error) {
        return (
            <div className="relative h-[calc(100vh-4rem)] overflow-hidden">
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center bg-gray-900/95 backdrop-blur-xl border border-gray-700/50 rounded-xl p-10 shadow-2xl max-w-md">
                        <AlertCircle className="w-12 h-12 text-amber-400 mx-auto mb-4" />
                        <h3 className="text-xl font-semibold text-white mb-2">Backend API unavailable</h3>
                        <p className="text-gray-400 mb-6 text-sm">
                            Could not load contract graph data. You can still explore the visualization using Demo Mode with synthetic data.
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium transition-colors inline-flex items-center gap-2"
                        >
                            <Sparkles className="w-4 h-4" />
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            </div>
        );
    }

    const nodes = graphData?.nodes ?? [];
    const edges = graphData?.edges ?? [];

    return (
        <div className="relative h-[calc(100vh-4rem)] overflow-hidden">
            {/* Graph Canvas */}
            <div className="w-full h-full bg-gray-900 dark:bg-gray-950 relative">
                <div className="absolute inset-0 flex items-center justify-center text-gray-400">
                    <div className="text-center">
                        <p className="text-lg font-medium mb-2">Contract Dependency Graph</p>
                        <p className="text-sm">Loading contract graph data...</p>
                    </div>
                </div>
            </div>

            {/* Controls Overlay */}
            <GraphControls
                searchQuery={searchQuery}
                onSearchChange={setSearchQuery}
                networkFilter={networkFilter}
                onNetworkFilterChange={setNetworkFilter}
                demoMode={demoMode}
                onDemoModeChange={setDemoMode}
                demoNodeCount={demoNodeCount}
                onDemoNodeCountChange={setDemoNodeCount}
                totalNodes={nodes.length}
                totalEdges={edges.length}
                criticalCount={criticalCount}
                searchMatchCount={searchMatches.length}
                searchMatchIndex={searchMatchIndex}
                onPrevMatch={handlePrevMatch}
                onNextMatch={handleNextMatch}
                onZoomIn={() => graphRef.current?.zoomIn()}
                onZoomOut={() => graphRef.current?.zoomOut()}
                onResetZoom={() => graphRef.current?.resetZoom()}
                onExportSVG={() => graphRef.current?.exportSVG()}
                onExportPNG={() => graphRef.current?.exportPNG()}
            />

            {/* Selected Node Panel */}
            {selectedNode && (
                <div className="absolute bottom-4 left-4 z-30 w-80 bg-gray-900/95 backdrop-blur-xl border border-gray-700/50 rounded-xl p-5 shadow-2xl animate-in slide-in-from-left-2">
                    <div className="flex items-start justify-between mb-3">
                        <h3 className="font-semibold text-white text-lg truncate pr-2">{selectedNode.name}</h3>
                        <button
                            onClick={() => setSelectedNode(null)}
                            className="text-gray-500 hover:text-gray-300 transition-colors shrink-0 text-lg leading-none"
                        >
                            ×
                        </button>
                    </div>
                    <p className="text-xs text-gray-500 font-mono mb-3 truncate">
                        {selectedNode.contract_id}
                    </p>
                    <div className="space-y-2 text-sm">
                        <div className="flex justify-between">
                            <span className="text-gray-400">Network</span>
                            <span className={`font-medium ${selectedNode.network === 'mainnet' ? 'text-green-400' :
                                selectedNode.network === 'testnet' ? 'text-blue-400' : 'text-purple-400'
                                }`}>{selectedNode.network}</span>
                        </div>
                        <div className="flex justify-between">
                            <span className="text-gray-400">Verified</span>
                            <span className={selectedNode.is_verified ? 'text-green-400' : 'text-gray-500'}>
                                {selectedNode.is_verified ? '✓ Yes' : 'No'}
                            </span>
                        </div>
                        {selectedNode.category && (
                            <div className="flex justify-between">
                                <span className="text-gray-400">Category</span>
                                <span className="text-gray-200">{selectedNode.category}</span>
                            </div>
                        )}
                        <div className="flex justify-between">
                            <span className="text-gray-400">Dependents</span>
                            <span className="text-gray-200">{dependentCounts.get(selectedNode.id) || 0}</span>
                        </div>
                        {selectedNode.tags.length > 0 && (
                            <div className="pt-2 border-t border-gray-800">
                                <span className="text-gray-400 text-xs">Tags</span>
                                <div className="flex flex-wrap gap-1 mt-1">
                                    {selectedNode.tags.map((tag) => (
                                        <span key={tag} className="px-2 py-0.5 bg-gray-800 text-gray-300 rounded text-xs">
                                            {tag}
                                        </span>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            )}

            {/* Empty State */}
            {!demoMode && nodes.length === 0 && !isLoading && (
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center bg-gray-900/80 backdrop-blur-xl border border-gray-700/50 rounded-xl p-10">
                        <Sparkles className="w-12 h-12 text-blue-400 mx-auto mb-4" />
                        <h3 className="text-xl font-semibold text-white mb-2">No contracts yet</h3>
                        <p className="text-gray-400 mb-4">
                            Publish some contracts or enable Demo Mode to explore the graph
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium transition-colors"
                        >
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}
