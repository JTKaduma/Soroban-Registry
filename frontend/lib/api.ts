import { MOCK_CONTRACTS, MOCK_EXAMPLES, MOCK_VERSIONS } from './mock-data';

export interface Contract {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: 'mainnet' | 'testnet' | 'futurenet';
  is_verified: boolean;
  category?: string;
  tags: string[];
  created_at: string;
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

export interface ContractSearchParams {
  query?: string;
  network?: 'mainnet' | 'testnet' | 'futurenet';
  verified_only?: boolean;
  category?: string;
  tags?: string[];
  page?: number;
  page_size?: number;
}

export interface PublishRequest {
  contract_id: string;
  name: string;
  description?: string;
  network: 'mainnet' | 'testnet' | 'futurenet';
  category?: string;
  tags: string[];
  source_url?: string;
  publisher_address: string;
}

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';
const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === 'true';

export const api = {
  // Contract endpoints
  async getContracts(params?: ContractSearchParams): Promise<PaginatedResponse<Contract>> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          let filtered = [...MOCK_CONTRACTS];
          
          if (params?.query) {
            const q = params.query.toLowerCase();
            filtered = filtered.filter(c => 
              c.name.toLowerCase().includes(q) || 
              (c.description && c.description.toLowerCase().includes(q))
            );
          }
          
          if (params?.category) {
            filtered = filtered.filter(c => c.category === params.category);
          }

          if (params?.verified_only) {
            filtered = filtered.filter(c => c.is_verified);
          }

          resolve({
            items: filtered,
            total: filtered.length,
            page: params?.page || 1,
            page_size: params?.page_size || 20,
            total_pages: 1
          });
        }, 500); 
      });
    }

    const queryParams = new URLSearchParams();
    if (params?.query) queryParams.append('query', params.query);
    if (params?.network) queryParams.append('network', params.network);
    if (params?.verified_only !== undefined) queryParams.append('verified_only', String(params.verified_only));
    if (params?.category) queryParams.append('category', params.category);
    if (params?.page) queryParams.append('page', String(params.page));
    if (params?.page_size) queryParams.append('page_size', String(params.page_size));

    const response = await fetch(`${API_URL}/api/contracts?${queryParams}`);
    if (!response.ok) throw new Error('Failed to fetch contracts');
    return response.json();
  },

  async getContract(id: string): Promise<Contract> {
    if (USE_MOCKS) {
      return new Promise((resolve, reject) => {
        setTimeout(() => {
          const contract = MOCK_CONTRACTS.find(c => c.id === id || c.contract_id === id);
          if (contract) {
            resolve(contract);
          } else {
            reject(new Error('Contract not found'));
          }
        }, 300);
      });
    }

    const response = await fetch(`${API_URL}/api/contracts/${id}`);
    if (!response.ok) throw new Error('Failed to fetch contract');
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
    if (!response.ok) throw new Error('Failed to fetch contract examples');
    return response.json();
  },

  async rateExample(id: string, userAddress: string, rating: number): Promise<ExampleRating> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve({
            id: 'mock-rating-id',
            example_id: id,
            user_address: userAddress,
            rating: rating,
            created_at: new Date().toISOString()
          });
        }, 300);
      });
    }

    const response = await fetch(`${API_URL}/api/examples/${id}/rate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_address: userAddress, rating }),
    });
    if (!response.ok) throw new Error('Failed to rate example');
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
    if (!response.ok) throw new Error('Failed to fetch contract versions');
    return response.json();
  },

  async publishContract(data: PublishRequest): Promise<Contract> {
    if (USE_MOCKS) {
      throw new Error('Publishing is not supported in mock mode');
    }

    const response = await fetch(`${API_URL}/api/contracts`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!response.ok) throw new Error('Failed to publish contract');
    return response.json();
  },

  // Publisher endpoints
  async getPublisher(id: string): Promise<Publisher> {
    if (USE_MOCKS) {
      return Promise.resolve({
          id: id,
          stellar_address: 'G...',
          username: 'Mock Publisher',
          created_at: new Date().toISOString()
      });
    }

    const response = await fetch(`${API_URL}/api/publishers/${id}`);
    if (!response.ok) throw new Error('Failed to fetch publisher');
    return response.json();
  },

  async getPublisherContracts(id: string): Promise<Contract[]> {
    if (USE_MOCKS) {
      return Promise.resolve(MOCK_CONTRACTS.filter(c => c.publisher_id === id));
    }

    const response = await fetch(`${API_URL}/api/publishers/${id}/contracts`);
    if (!response.ok) throw new Error('Failed to fetch publisher contracts');
    return response.json();
  },

  // Stats endpoint
  async getStats(): Promise<{ total_contracts: number; verified_contracts: number; total_publishers: number }> {
    if (USE_MOCKS) {
       return Promise.resolve({
           total_contracts: MOCK_CONTRACTS.length,
           verified_contracts: MOCK_CONTRACTS.filter(c => c.is_verified).length,
           total_publishers: 5
       });
    }

    const response = await fetch(`${API_URL}/api/stats`);
    if (!response.ok) throw new Error('Failed to fetch stats');
    return response.json();
  },
};

export interface ContractExample {
  id: string;
  contract_id: string;
  title: string;
  description?: string;
  code_rust?: string;
  code_js?: string;
  category?: 'basic' | 'advanced' | 'integration';
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
