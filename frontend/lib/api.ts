import {
  MOCK_CONTRACTS,
  MOCK_EXAMPLES,
  MOCK_VERSIONS,
  MOCK_TEMPLATES,
} from "./mock-data";

export interface Contract {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: "mainnet" | "testnet" | "futurenet";
  is_verified: boolean;
  category?: string;
  tags: string[];
  popularity_score?: number;
  downloads?: number;
  created_at: string;
  updated_at: string;
  is_maintenance?: boolean;
  maturity?: 'alpha' | 'beta' | 'stable' | 'mature' | 'legacy';
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
  sort_by?: "name" | "created_at" | "popularity" | "downloads";
  sort_order?: "asc" | "desc";
  tags?: string[];
  maturity?: 'alpha' | 'beta' | 'stable' | 'mature' | 'legacy';
  page?: number;
  page_size?: number;
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
    if (params?.sort_by) queryParams.append("sort_by", params.sort_by);
    if (params?.sort_order) queryParams.append("sort_order", params.sort_order);
    if (params?.page) queryParams.append("page", String(params.page));
    if (params?.page_size)
      queryParams.append("page_size", String(params.page_size));

    const response = await fetch(`${API_URL}/api/contracts?${queryParams}`);
    if (!response.ok) throw new Error("Failed to fetch contracts");
    return response.json();
  },

  async getContract(id: string): Promise<Contract> {
    if (USE_MOCKS) {
      return new Promise((resolve, reject) => {
        setTimeout(() => {
          const contract = MOCK_CONTRACTS.find(
            (c) => c.id === id || c.contract_id === id,
          );
          if (contract) {
            resolve(contract);
          } else {
            reject(new Error("Contract not found"));
          }
        }, 300);
      });
    }

    const response = await fetch(`${API_URL}/api/contracts/${id}`);
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

  // Graph endpoint
  async getContractGraph(network?: string): Promise<GraphResponse> {
    const queryParams = new URLSearchParams();
    if (network) queryParams.append("network", network);
    const qs = queryParams.toString();
    const response = await fetch(`${API_URL}/api/contracts/graph${qs ? `?${qs}` : ""}`);
    if (!response.ok) throw new Error("Failed to fetch contract graph");
    return response.json();
  },

  async getContractDependencies(id: string): Promise<any> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/dependencies`);
    if (!response.ok) throw new Error('Failed to fetch contract dependencies');
    return response.json();
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

export interface MaintenanceWindow {
  id: string;
  contract_id: string;
  message: string;
  started_at: string;
  scheduled_end_at?: string;
  ended_at?: string;
  created_by: string;
  created_at: string;
}

export interface MaintenanceStatus {
  is_maintenance: boolean;
  current_window?: MaintenanceWindow;
}

export const maintenanceApi = {
  async getStatus(contractId: string): Promise<MaintenanceStatus> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maintenance`);
    if (!response.ok) throw new Error('Failed to fetch maintenance status');
    return response.json();
  },

  async getHistory(contractId: string): Promise<MaintenanceWindow[]> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maintenance/history`);
    if (!response.ok) throw new Error('Failed to fetch maintenance history');
    return response.json();
  },

  async start(contractId: string, message: string, scheduledEndAt?: string): Promise<MaintenanceWindow> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maintenance`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, scheduled_end_at: scheduledEndAt }),
    });
    if (!response.ok) throw new Error('Failed to start maintenance');
    return response.json();
  },

  async end(contractId: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maintenance`, {
      method: 'DELETE',
    });
    if (!response.ok) throw new Error('Failed to end maintenance');
  },
};

export type MaturityLevel = 'alpha' | 'beta' | 'stable' | 'mature' | 'legacy';

export interface MaturityChange {
  id: string;
  contract_id: string;
  from_level?: MaturityLevel;
  to_level: MaturityLevel;
  reason?: string;
  changed_by: string;
  changed_at: string;
}

export interface MaturityCriterion {
  name: string;
  required: boolean;
  met: boolean;
  description: string;
}

export interface MaturityRequirements {
  level: MaturityLevel;
  criteria: MaturityCriterion[];
  met: boolean;
}

export const maturityApi = {
  async update(contractId: string, maturity: MaturityLevel, reason?: string): Promise<Contract> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maturity`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ maturity, reason }),
    });
    if (!response.ok) throw new Error('Failed to update maturity');
    return response.json();
  },

  async getHistory(contractId: string): Promise<MaturityChange[]> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maturity/history`);
    if (!response.ok) throw new Error('Failed to fetch maturity history');
    return response.json();
  },

  async checkRequirements(contractId: string): Promise<MaturityRequirements[]> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/maturity/requirements`);
    if (!response.ok) throw new Error('Failed to check requirements');
    return response.json();
  },
};

export interface CostEstimateRequest {
  method_name: string;
  invocations?: number;
  storage_growth_kb?: number;
}

export interface CostEstimate {
  method_name: string;
  gas_cost: number;
  storage_cost: number;
  bandwidth_cost: number;
  total_stroops: number;
  total_xlm: number;
  invocations: number;
}

export interface BatchCostEstimate {
  estimates: CostEstimate[];
  total_stroops: number;
  total_xlm: number;
}

export interface CostOptimization {
  current_cost: number;
  optimized_cost: number;
  savings_percent: number;
  suggestions: string[];
}

export interface CostForecast {
  daily_cost_xlm: number;
  monthly_cost_xlm: number;
  yearly_cost_xlm: number;
  usage_pattern: string;
}

export const costApi = {
  async estimate(contractId: string, request: CostEstimateRequest): Promise<CostEstimate> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/cost-estimate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    if (!response.ok) throw new Error('Failed to estimate cost');
    return response.json();
  },

  async batchEstimate(contractId: string, requests: CostEstimateRequest[]): Promise<BatchCostEstimate> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/cost-estimate/batch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requests),
    });
    if (!response.ok) throw new Error('Failed to batch estimate');
    return response.json();
  },

  async optimize(contractId: string, estimate: CostEstimate): Promise<CostOptimization> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/cost-estimate/optimize`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(estimate),
    });
    if (!response.ok) throw new Error('Failed to optimize costs');
    return response.json();
  },

  async forecast(contractId: string, request: CostEstimateRequest): Promise<CostForecast> {
    const response = await fetch(`${API_URL}/api/contracts/${contractId}/cost-estimate/forecast`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    if (!response.ok) throw new Error('Failed to forecast costs');
    return response.json();
  },
};
