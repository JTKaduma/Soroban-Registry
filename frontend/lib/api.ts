import {
  MOCK_CONTRACTS,
  MOCK_EXAMPLES,
  MOCK_VERSIONS,
  MOCK_TEMPLATES,
} from "./mock-data";

export type Network = "mainnet" | "testnet" | "futurenet";

/** Per-network config (Issue #43) */
export interface NetworkConfig {
  contract_id: string;
  is_verified: boolean;
  min_version?: string;
  max_version?: string;
}

export interface Contract {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: Network;
  is_verified: boolean;
  category?: string;
  tags: string[];
  popularity_score?: number;
  downloads?: number;
  created_at: string;
  updated_at: string;
  is_maintenance?: boolean;
  /** Logical contract grouping (Issue #43) */
  logical_id?: string;
  /** Per-network configs: { mainnet: {...}, testnet: {...} } */
  network_configs?: Record<Network, NetworkConfig>;
}

/** GET /contracts/:id response when ?network= is used (Issue #43) */
export interface ContractGetResponse extends Contract {
  current_network?: Network;
  network_config?: NetworkConfig;
}

export interface ContractHealth {
  contract_id: string;
  status: "healthy" | "warning" | "critical";
  last_activity: string;
  security_score: number;
  audit_date?: string;
  total_score: number;
  recommendations: string[];
  updated_at: string;
}

/** Contract interaction (invocation) — Issue #46 */
export interface ContractInteractionResponse {
  id: string;
  account: string | null;
  method: string | null;
  parameters: unknown;
  return_value: unknown;
  transaction_hash: string | null;
  created_at: string;
}

export interface InteractionsQueryParams {
  limit?: number;
  offset?: number;
  account?: string;
  method?: string;
  from_timestamp?: string;
  to_timestamp?: string;
}

export interface InteractionsListResponse {
  items: ContractInteractionResponse[];
  total: number;
  limit: number;
  offset: number;
}

/** Analytics timeline entry (one day) */
export interface TimelineEntry {
  date: string;
  count: number;
}

export interface TopUser {
  address: string;
  count: number;
}

export interface InteractorStats {
  unique_count: number;
  top_users: TopUser[];
}

export interface DeploymentStats {
  count: number;
  unique_users: number;
  by_network: Record<string, number>;
}

export interface ContractAnalyticsResponse {
  contract_id: string;
  deployments: DeploymentStats;
  interactors: InteractorStats;
  timeline: TimelineEntry[];
}

export interface ContractVersion {
  id: string;
  contract_id: string;
  version: string;
  wasm_hash: string;
  source_url?: string;
  commit_hash?: string;
  release_notes?: string;
  created_at: string;
}

export interface Publisher {
  id: string;
  stellar_address: string;
  username?: string;
  email?: string;
  github_url?: string;
  website?: string;
  created_at: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface DependencyTreeNode {
  contract_id: string;
  name: string;
  current_version: string;
  constraint_to_parent: string;
  dependencies: DependencyTreeNode[];
}

export interface MaintenanceWindow {
  message: string;
  scheduled_end_at?: string;
}

export type MaturityLevel = 'alpha' | 'beta' | 'stable' | 'mature' | 'legacy';

export interface ContractSearchParams {
  query?: string;
  network?: "mainnet" | "testnet" | "futurenet";
  networks?: Array<"mainnet" | "testnet" | "futurenet">;
  verified_only?: boolean;
  category?: string;
  categories?: string[];
  language?: string;
  languages?: string[];
  author?: string;
  tags?: string[];
  maturity?: 'alpha' | 'beta' | 'stable' | 'mature' | 'legacy';
  page?: number;
  page_size?: number;
  sort_by?: 'name' | 'created_at' | 'updated_at' | 'popularity' | 'deployments' | 'interactions' | 'relevance' | 'downloads';
  sort_order?: 'asc' | 'desc';
}

export interface PublishRequest {
  contract_id: string;
  name: string;
  description?: string;
  network: "mainnet" | "testnet" | "futurenet";
  category?: string;
  tags: string[];
  source_url?: string;
  publisher_address: string;
}

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";
const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === "true";

export const api = {
  // Contract endpoints
  async getContracts(
    params?: ContractSearchParams,
  ): Promise<PaginatedResponse<Contract>> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          let filtered = [...MOCK_CONTRACTS];

          if (params?.query) {
            const q = params.query.toLowerCase();
            filtered = filtered.filter(
              (c) =>
                c.name.toLowerCase().includes(q) ||
                (c.description && c.description.toLowerCase().includes(q)) ||
                c.tags.some((tag) => tag.toLowerCase().includes(q)),
            );
          }

          const categories = params?.categories?.length
            ? params.categories
            : params?.category
              ? [params.category]
              : [];
          if (categories.length > 0) {
            filtered = filtered.filter(
              (c) => c.category && categories.includes(c.category),
            );
          }

          const networks = params?.networks?.length
            ? params.networks
            : params?.network
              ? [params.network]
              : [];
          if (networks.length > 0) {
            filtered = filtered.filter((c) => networks.includes(c.network));
          }

          const languages = params?.languages?.length
            ? params.languages
            : params?.language
              ? [params.language]
              : [];
          if (languages.length > 0) {
            const normalized = languages.map((language) => language.toLowerCase());
            filtered = filtered.filter((c) =>
              c.tags.some((tag) => normalized.includes(tag.toLowerCase())),
            );
          }

          if (params?.author) {
            const author = params.author.toLowerCase();
            filtered = filtered.filter((c) =>
              c.publisher_id.toLowerCase().includes(author),
            );
          }

          if (params?.verified_only) {
            filtered = filtered.filter((c) => c.is_verified);
          }

          const sortBy = params?.sort_by || "created_at";
          const sortOrder = params?.sort_order || "desc";
          filtered.sort((a, b) => {
            if (sortBy === "name") {
              return a.name.localeCompare(b.name);
            }
            if (sortBy === "popularity") {
              const aPopularity = a.popularity_score ?? 0;
              const bPopularity = b.popularity_score ?? 0;
              return aPopularity - bPopularity;
            }
            if (sortBy === "downloads") {
              const aDownloads = a.downloads ?? 0;
              const bDownloads = b.downloads ?? 0;
              return aDownloads - bDownloads;
            }
            return (
              new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
            );
          });
          if (sortOrder === "desc") {
            filtered.reverse();
          }

          const page = params?.page || 1;
          const pageSize = params?.page_size || 20;
          const start = (page - 1) * pageSize;
          const end = start + pageSize;
          const items = filtered.slice(start, end);

          resolve({
            items,
            total: filtered.length,
            page,
            page_size: pageSize,
            total_pages: Math.max(1, Math.ceil(filtered.length / pageSize)),
          });
        }, 500);
      });
    }

    const queryParams = new URLSearchParams();
    if (params?.query) queryParams.append("query", params.query);
    if (params?.network) queryParams.append("network", params.network);
    params?.networks?.forEach((network) => queryParams.append("network", network));
    if (params?.verified_only !== undefined)
      queryParams.append("verified_only", String(params.verified_only));
    if (params?.category) queryParams.append("category", params.category);
    params?.categories?.forEach((category) =>
      queryParams.append("category", category),
    );
    if (params?.language) queryParams.append("language", params.language);
    params?.languages?.forEach((language) =>
      queryParams.append("language", language),
    );
    if (params?.author) queryParams.append("author", params.author);
    // Backend expects sort_by without underscores: createdat, updatedat, popularity, deployments, interactions, relevance
    if (params?.sort_by) {
      const backendSortBy =
        params.sort_by === 'created_at' ? 'createdat'
        : params.sort_by === 'updated_at' ? 'updatedat'
        : params.sort_by === 'name' ? 'createdat'
        : params.sort_by === 'downloads' ? 'interactions'
        : params.sort_by;
      queryParams.append("sort_by", backendSortBy);
    }
    if (params?.sort_order) queryParams.append("sort_order", params.sort_order);
    if (params?.page) queryParams.append("page", String(params.page));
    if (params?.page_size)
      queryParams.append("page_size", String(params.page_size));

    const response = await fetch(`${API_URL}/api/contracts?${queryParams}`);
    if (!response.ok) throw new Error("Failed to fetch contracts");
    const data = await response.json();
    // Backend returns "contracts"; normalize to "items" for PaginatedResponse
    if (Array.isArray(data.contracts) && data.items === undefined) {
      return { ...data, items: data.contracts };
    }
    return data;
  },

  async getContract(id: string, network?: Network): Promise<ContractGetResponse> {
    if (USE_MOCKS) {
      return new Promise((resolve, reject) => {
        setTimeout(() => {
          const contract = MOCK_CONTRACTS.find(
            (c) => c.id === id || c.contract_id === id,
          );
          if (contract) {
            resolve(contract as ContractGetResponse);
          } else {
            reject(new Error("Contract not found"));
          }
        }, 300);
      });
    }

    const url = new URL(`${API_URL}/api/contracts/${id}`);
    if (network) url.searchParams.set("network", network);
    const response = await fetch(url.toString());
    if (!response.ok) throw new Error("Failed to fetch contract");
    return response.json();
  },

  async getContractExamples(id: string): Promise<ContractExample[]> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve(MOCK_EXAMPLES[id] || []);
        }, 500);
      });
    }

    const response = await fetch(`${API_URL}/api/contracts/${id}/examples`);
    if (!response.ok) throw new Error("Failed to fetch contract examples");
    return response.json();
  },

  async rateExample(
    id: string,
    userAddress: string,
    rating: number,
  ): Promise<ExampleRating> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve({
            id: "mock-rating-id",
            example_id: id,
            user_address: userAddress,
            rating: rating,
            created_at: new Date().toISOString(),
          });
        }, 300);
      });
    }

    const response = await fetch(`${API_URL}/api/examples/${id}/rate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ user_address: userAddress, rating }),
    });
    if (!response.ok) throw new Error("Failed to rate example");
    return response.json();
  },

  async getContractVersions(id: string): Promise<ContractVersion[]> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve(MOCK_VERSIONS[id] || []);
        }, 300);
      });
    }

    const response = await fetch(`${API_URL}/api/contracts/${id}/versions`);
    if (!response.ok) throw new Error("Failed to fetch contract versions");
    return response.json();
  },

  async getContractDependencies(id: string): Promise<DependencyTreeNode[]> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/dependencies`);
    if (!response.ok) throw new Error('Failed to fetch contract dependencies');
    return response.json();
  },

  async getContractInteractions(
    id: string,
    params?: InteractionsQueryParams,
  ): Promise<InteractionsListResponse> {
    const search = new URLSearchParams();
    if (params?.limit != null) search.set("limit", String(params.limit));
    if (params?.offset != null) search.set("offset", String(params.offset));
    if (params?.account) search.set("account", params.account);
    if (params?.method) search.set("method", params.method);
    if (params?.from_timestamp) search.set("from_timestamp", params.from_timestamp);
    if (params?.to_timestamp) search.set("to_timestamp", params.to_timestamp);
    const qs = search.toString();
    const response = await fetch(
      `${API_URL}/api/contracts/${id}/interactions${qs ? `?${qs}` : ""}`,
    );
    if (!response.ok) throw new Error("Failed to fetch contract interactions");
    return response.json();
  },

  async getContractAnalytics(id: string): Promise<ContractAnalyticsResponse> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/analytics`);
    if (!response.ok) throw new Error("Failed to fetch contract analytics");
    return response.json();
  },

  async publishContract(data: PublishRequest): Promise<Contract> {
    if (USE_MOCKS) {
      throw new Error("Publishing is not supported in mock mode");
    }

    const response = await fetch(`${API_URL}/api/contracts`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    if (!response.ok) throw new Error("Failed to publish contract");
    return response.json();
  },

  async getContractHealth(id: string): Promise<ContractHealth> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/health`);
    if (!response.ok) throw new Error("Failed to fetch contract health");
    return response.json();
  },

  async getFormalVerificationResults(id: string): Promise<FormalVerificationReport[]> {
    if (USE_MOCKS) {
      return Promise.resolve([]);
    }
    const response = await fetch(`${API_URL}/api/contracts/${id}/formal-verification`);
    if (!response.ok) throw new Error('Failed to fetch formal verification results');
    return response.json();
  },

  async runFormalVerification(id: string, data: RunVerificationRequest): Promise<FormalVerificationReport> {
    if (USE_MOCKS) {
      throw new Error('Formal verification is not supported in mock mode');
    }
    const response = await fetch(`${API_URL}/api/contracts/${id}/formal-verification`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!response.ok) throw new Error('Failed to run formal verification');
    return response.json();
  },

  // Publisher endpoints
  async getPublisher(id: string): Promise<Publisher> {
    if (USE_MOCKS) {
      return Promise.resolve({
        id: id,
        stellar_address: "G...",
        username: "Mock Publisher",
        created_at: new Date().toISOString(),
      });
    }

    const response = await fetch(`${API_URL}/api/publishers/${id}`);
    if (!response.ok) throw new Error("Failed to fetch publisher");
    return response.json();
  },

  async getPublisherContracts(id: string): Promise<Contract[]> {
    if (USE_MOCKS) {
      return Promise.resolve(
        MOCK_CONTRACTS.filter((c) => c.publisher_id === id),
      );
    }

    const response = await fetch(`${API_URL}/api/publishers/${id}/contracts`);
    if (!response.ok) throw new Error("Failed to fetch publisher contracts");
    return response.json();
  },

  async getStats(): Promise<{
    total_contracts: number;
    verified_contracts: number;
    total_publishers: number;
  }> {
    if (USE_MOCKS) {
      return Promise.resolve({
        total_contracts: MOCK_CONTRACTS.length,
        verified_contracts: MOCK_CONTRACTS.filter((c) => c.is_verified).length,
        total_publishers: 5,
      });
    }

    const response = await fetch(`${API_URL}/api/stats`);
    if (!response.ok) throw new Error("Failed to fetch stats");
    return response.json();
  },

  // Compatibility endpoints
  async getCompatibility(id: string): Promise<CompatibilityMatrix> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/compatibility`);
    if (!response.ok) throw new Error('Failed to fetch compatibility matrix');
    return response.json();
  },

  async addCompatibility(id: string, data: AddCompatibilityRequest): Promise<unknown> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/compatibility`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!response.ok) throw new Error('Failed to add compatibility entry');
    return response.json();
  },

  getCompatibilityExportUrl(id: string, format: 'csv' | 'json'): string {
    return `${API_URL}/api/contracts/${id}/compatibility/export?format=${format}`;
  },

  // Graph endpoint (backend may return { graph: {} } or { nodes, edges }; normalize to GraphResponse)
  async getContractGraph(network?: string): Promise<GraphResponse> {
    const queryParams = new URLSearchParams();
    if (network) queryParams.append("network", network);
    const qs = queryParams.toString();

    const response = await fetch(`${API_URL}/api/contracts/graph${qs ? `?${qs}` : ""}`);
    if (!response.ok) throw new Error("Failed to fetch contract graph");
    const data = await response.json();
    const raw = data?.graph ?? data;
    return {
      nodes: Array.isArray(raw?.nodes) ? raw.nodes : [],
      edges: Array.isArray(raw?.edges) ? raw.edges : [],
    };
  },

  async getTemplates(): Promise<Template[]> {
    if (USE_MOCKS) {
      return Promise.resolve([]);
    }
    const response = await fetch(`${API_URL}/api/templates`);
    if (!response.ok) throw new Error('Failed to fetch templates');

    return response.json();
  },
};

export interface Template {
  id: string;
  slug: string;
  name: string;
  description?: string;
  category: string;
  version: string;
  install_count: number;
  parameters: {
    name: string;
    type: string;
    default?: string;
    description?: string;
  }[];
  created_at: string;
}

export interface GraphNode {
  id: string;
  contract_id: string;
  name: string;
  network: "mainnet" | "testnet" | "futurenet";
  is_verified: boolean;
  category?: string;
  tags: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  dependency_type: string;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
}


export interface ContractExample {
  id: string;
  contract_id: string;
  title: string;
  description?: string;
  code_rust?: string;
  code_js?: string;
  category?: "basic" | "advanced" | "integration";
  rating_up: number;
  rating_down: number;
  created_at: string;
  updated_at: string;
}

export interface ExampleRating {
  id: string;
  example_id: string;
  user_address: string;
  rating: number;
  created_at: string;
}

// ─── Compatibility Matrix ────────────────────────────────────────────────────

export interface CompatibilityEntry {
  target_contract_id: string;
  target_contract_stellar_id: string;
  target_contract_name: string;
  target_version: string;
  stellar_version?: string;
  is_compatible: boolean;
}

/** Shape returned by GET /api/contracts/:id/compatibility */
export interface CompatibilityMatrix {
  contract_id: string;
  /** Keyed by source version string */
  versions: Record<string, CompatibilityEntry[]>;
  warnings: string[];
  total_entries: number;
}

export interface AddCompatibilityRequest {
  source_version: string;
  target_contract_id: string;
  target_version: string;
  stellar_version?: string;
  is_compatible: boolean;
}

// ─── Formal Verification ─────────────────────────────────────────────────────

export type VerificationStatus = 'Proved' | 'Violated' | 'Unknown' | 'Skipped';

export interface FormalVerificationSession {
  id: string;
  contract_id: string;
  version: string;
  verifier_version: string;
  created_at: string;
  updated_at: string;
}

export interface FormalVerificationProperty {
  id: string;
  session_id: string;
  property_id: string;
  description?: string;
  invariant: string;
  severity: string;
}

export interface FormalVerificationResult {
  id: string;
  property_id: string;
  status: VerificationStatus;
  counterexample?: string;
  details?: string;
}

export interface FormalVerificationPropertyResult {
  property: FormalVerificationProperty;
  result: FormalVerificationResult;
}

export interface FormalVerificationReport {
  session: FormalVerificationSession;
  properties: FormalVerificationPropertyResult[];
}

export interface RunVerificationRequest {
  properties_file: string;
  verifier_version?: string;
}
