# API Rate Limiting and Quota Policy

## Overview

The Soroban Registry API implements rate limiting to ensure fair usage, prevent abuse, and maintain service quality for all users. Rate limits are applied per IP address and per endpoint, with different tiers based on request type.

## Rate Limit Tiers

### Default Rate Limits (Per Minute)

| Tier | Limit | Description |
|------|-------|-------------|
| **Read Operations (GET)** | 100 requests/min | Standard read operations (contract search, retrieval, etc.) |
| **Write Operations (POST/PUT/PATCH/DELETE)** | 20 requests/min | Contract publishing, updates, deletions |
| **Authenticated Requests** | 1,000 requests/min | Requests with valid `Authorization` header |
| **Health Checks** | 10,000 requests/min | `/health` endpoint for monitoring |

### Endpoint-Specific Limits

You can configure custom limits for specific endpoints using environment variables:

```bash
RATE_LIMIT_ENDPOINT_POST_CONTRACTS_VERIFY=10
RATE_LIMIT_ENDPOINT_GET_CONTRACTS_SEARCH=200
```

The endpoint key format is: `{METHOD}_{NORMALIZED_PATH}` (e.g., `POST_CONTRACTS_VERIFY`).

## Rate Limit Headers

Every API response includes rate limit information in the following headers:

| Header | Description | Example |
|--------|-------------|---------|
| `X-RateLimit-Limit` | Maximum requests allowed in the current window | `100` |
| `X-RateLimit-Remaining` | Remaining requests in the current window | `73` |
| `X-RateLimit-Reset` | Seconds until the rate limit window resets | `42` |
| `Retry-After` | *(Only on 429)* Seconds to wait before retrying | `42` |

### Example Response Headers

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 42
Content-Type: application/json
```

### Rate Limit Exceeded (429 Response)

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 42
Retry-After: 42
Content-Type: application/json

{
  "error": "RateLimitExceeded",
  "message": "Too many requests. Please retry after the indicated time.",
  "code": 429,
  "timestamp": "2026-02-24T12:34:56Z",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

## Retry Strategy

### Exponential Backoff with Jitter

When you receive a `429 Too Many Requests` response, implement exponential backoff with jitter to avoid thundering herd problems:

**Formula**: `wait_time = min(max_wait, base_delay * (2 ^ attempt)) + random_jitter`

### Python Example

```python
import time
import random
import requests
from typing import Optional

def call_api_with_retry(
    url: str,
    max_retries: int = 5,
    base_delay: float = 1.0,
    max_delay: float = 60.0
) -> Optional[requests.Response]:
    """
    Call API with exponential backoff retry strategy.

    Args:
        url: API endpoint URL
        max_retries: Maximum number of retry attempts
        base_delay: Initial delay in seconds
        max_delay: Maximum delay between retries

    Returns:
        Response object or None if all retries exhausted
    """
    for attempt in range(max_retries + 1):
        try:
            response = requests.get(url)

            # Check rate limit headers proactively
            remaining = int(response.headers.get('X-RateLimit-Remaining', 1))
            if remaining <= 5:
                print(f"Warning: Only {remaining} requests remaining in current window")

            if response.status_code == 429:
                # Use Retry-After header if available
                retry_after = int(response.headers.get('Retry-After', 0))
                if retry_after > 0:
                    wait_time = retry_after
                else:
                    # Calculate exponential backoff with jitter
                    wait_time = min(max_delay, base_delay * (2 ** attempt))
                    jitter = random.uniform(0, wait_time * 0.1)
                    wait_time += jitter

                print(f"Rate limited. Waiting {wait_time:.2f}s before retry {attempt + 1}/{max_retries}")
                time.sleep(wait_time)
                continue

            # Success or non-retryable error
            response.raise_for_status()
            return response

        except requests.RequestException as e:
            if attempt == max_retries:
                print(f"Max retries exhausted: {e}")
                return None

            # Exponential backoff for network errors
            wait_time = min(max_delay, base_delay * (2 ** attempt))
            jitter = random.uniform(0, wait_time * 0.1)
            wait_time += jitter

            print(f"Request failed: {e}. Retrying in {wait_time:.2f}s...")
            time.sleep(wait_time)

    return None

# Usage
response = call_api_with_retry('https://registry.soroban.example/api/contracts/search?query=token')
if response:
    data = response.json()
    print(f"Found {len(data['contracts'])} contracts")
```

### JavaScript/TypeScript Example

```typescript
interface RetryConfig {
  maxRetries?: number;
  baseDelay?: number;
  maxDelay?: number;
}

interface RateLimitHeaders {
  limit: number;
  remaining: number;
  reset: number;
}

async function callApiWithRetry(
  url: string,
  options: RequestInit = {},
  config: RetryConfig = {}
): Promise<Response> {
  const {
    maxRetries = 5,
    baseDelay = 1000,
    maxDelay = 60000
  } = config;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const response = await fetch(url, options);

      // Parse rate limit headers
      const rateLimitHeaders: RateLimitHeaders = {
        limit: parseInt(response.headers.get('X-RateLimit-Limit') || '0'),
        remaining: parseInt(response.headers.get('X-RateLimit-Remaining') || '0'),
        reset: parseInt(response.headers.get('X-RateLimit-Reset') || '0')
      };

      // Proactive rate limit warning
      if (rateLimitHeaders.remaining <= 5) {
        console.warn(
          `Rate limit warning: ${rateLimitHeaders.remaining} requests remaining. ` +
          `Resets in ${rateLimitHeaders.reset}s`
        );
      }

      if (response.status === 429) {
        const retryAfter = parseInt(response.headers.get('Retry-After') || '0');
        let waitTime: number;

        if (retryAfter > 0) {
          waitTime = retryAfter * 1000; // Convert to milliseconds
        } else {
          // Exponential backoff with jitter
          waitTime = Math.min(maxDelay, baseDelay * Math.pow(2, attempt));
          const jitter = Math.random() * waitTime * 0.1;
          waitTime += jitter;
        }

        console.log(
          `Rate limited. Waiting ${(waitTime / 1000).toFixed(2)}s before ` +
          `retry ${attempt + 1}/${maxRetries}`
        );

        await new Promise(resolve => setTimeout(resolve, waitTime));
        continue;
      }

      // Success or non-retryable error
      return response;

    } catch (error) {
      if (attempt === maxRetries) {
        throw new Error(`Max retries exhausted: ${error}`);
      }

      // Exponential backoff for network errors
      const waitTime = Math.min(maxDelay, baseDelay * Math.pow(2, attempt));
      const jitter = Math.random() * waitTime * 0.1;
      const totalWait = waitTime + jitter;

      console.log(`Request failed: ${error}. Retrying in ${(totalWait / 1000).toFixed(2)}s...`);
      await new Promise(resolve => setTimeout(resolve, totalWait));
    }
  }

  throw new Error('Max retries exhausted');
}

// Usage
try {
  const response = await callApiWithRetry(
    'https://registry.soroban.example/api/contracts/search?query=token',
    {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json'
      }
    }
  );

  const data = await response.json();
  console.log(`Found ${data.contracts.length} contracts`);
} catch (error) {
  console.error('API call failed:', error);
}
```

### Rust Example

```rust
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

pub async fn call_api_with_retry(
    client: &Client,
    url: &str,
    config: RetryConfig,
) -> Result<Response, Box<dyn std::error::Error>> {
    for attempt in 0..=config.max_retries {
        let response = client.get(url).send().await?;

        // Parse rate limit headers
        let remaining: u32 = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        if remaining <= 5 {
            tracing::warn!(
                remaining,
                "Rate limit warning: few requests remaining"
            );
        }

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after: u64 = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let wait_ms = if retry_after > 0 {
                retry_after * 1000
            } else {
                // Exponential backoff with jitter
                let base_wait = config.base_delay_ms * 2u64.pow(attempt);
                let wait = std::cmp::min(config.max_delay_ms, base_wait);
                let jitter = (rand::random::<f64>() * wait as f64 * 0.1) as u64;
                wait + jitter
            };

            tracing::info!(
                attempt,
                wait_ms,
                max_retries = config.max_retries,
                "Rate limited, waiting before retry"
            );

            sleep(Duration::from_millis(wait_ms)).await;
            continue;
        }

        // Success or non-retryable error
        response.error_for_status_ref()?;
        return Ok(response);
    }

    Err("Max retries exhausted".into())
}

// Usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let config = RetryConfig::default();

    let response = call_api_with_retry(
        &client,
        "https://registry.soroban.example/api/contracts/search?query=token",
        config,
    ).await?;

    let data: serde_json::Value = response.json().await?;
    println!("Found contracts: {}", data["contracts"].as_array().unwrap().len());

    Ok(())
}
```

## Best Practices

### 1. Monitor Rate Limit Headers Proactively

Don't wait for a `429` response. Check `X-RateLimit-Remaining` on every successful response and slow down when you're close to the limit:

```python
remaining = int(response.headers.get('X-RateLimit-Remaining', 100))
reset_seconds = int(response.headers.get('X-RateLimit-Reset', 60))

if remaining <= 10:
    # Slow down: add delay between requests
    delay = reset_seconds / remaining if remaining > 0 else reset_seconds
    time.sleep(delay)
```

### 2. Use Connection Pooling

Reuse HTTP connections to reduce overhead:

```python
# Python
session = requests.Session()
session.headers.update({'User-Agent': 'MyApp/1.0'})

# JavaScript/Node.js
const agent = new https.Agent({ keepAlive: true });
```

### 3. Implement Request Batching

When possible, use batch endpoints to reduce request count:

```bash
# Instead of multiple requests:
GET /api/contracts/{id1}
GET /api/contracts/{id2}
GET /api/contracts/{id3}

# Use batch endpoint:
POST /api/contracts/batch
{
  "contract_ids": ["id1", "id2", "id3"]
}
```

### 4. Cache Responses

Cache API responses locally to avoid redundant requests:

```python
from functools import lru_cache
import time

@lru_cache(maxsize=1000)
def get_contract(contract_id: str, cache_key: int) -> dict:
    """Cache contract data with time-based invalidation"""
    response = requests.get(f'/api/contracts/{contract_id}')
    return response.json()

# Use with cache key that changes every 5 minutes
cache_key = int(time.time() / 300)
contract = get_contract('contract_abc123', cache_key)
```

### 5. Distribute Load Across Time

For bulk operations, spread requests over time:

```python
import asyncio

async def process_contracts_gradually(contract_ids: list[str]):
    """Process contracts with controlled rate"""
    for batch in chunks(contract_ids, size=10):
        tasks = [fetch_contract(cid) for cid in batch]
        await asyncio.gather(*tasks)
        # Wait between batches to respect rate limits
        await asyncio.sleep(6)  # 10 requests per minute = 1 batch per 6 seconds
```

## Configuration via Environment Variables

Rate limits can be customized using environment variables:

```bash
# Global limits (per minute)
RATE_LIMIT_READ_PER_MINUTE=100          # Default: 100
RATE_LIMIT_WRITE_PER_MINUTE=20          # Default: 20
RATE_LIMIT_AUTH_PER_MINUTE=1000         # Default: 1000
RATE_LIMIT_HEALTH_PER_MINUTE=10000      # Default: 10000

# Time window in seconds
RATE_LIMIT_WINDOW_SECONDS=60            # Default: 60

# Per-endpoint overrides
RATE_LIMIT_ENDPOINT_POST_CONTRACTS_VERIFY=10
RATE_LIMIT_ENDPOINT_GET_CONTRACTS_SEARCH=200
```

## FAQ

### Q: Are rate limits per user or per IP address?
**A:** Rate limits are applied **per IP address** and **per endpoint**. If multiple users share the same IP (e.g., behind a corporate NAT), they share the same rate limit. Authenticated requests (with `Authorization` header) receive higher limits.

### Q: Do rate limits apply to health check endpoints?
**A:** Yes, but with a much higher limit (10,000 requests/min by default) to support monitoring systems.

### Q: What happens if I exceed the rate limit?
**A:** You'll receive a `429 Too Many Requests` response with `Retry-After` header indicating when you can retry. Your request is not processed.

### Q: Can I request a higher rate limit?
**A:** For production deployments or high-volume integrations, contact the registry operators to discuss enterprise tier access with custom limits.

### Q: Do failed requests count toward the rate limit?
**A:** Yes, all requests (successful or failed) count toward your rate limit to prevent abuse through intentionally malformed requests.

### Q: How do I authenticate to get higher limits?
**A:** Include an `Authorization` header with a valid API key (when authentication is implemented). Authenticated requests automatically receive the higher authenticated tier limit (1,000 req/min).

### Q: Are WebSocket connections rate limited?
**A:** WebSocket connections are not currently supported. Rate limiting applies only to HTTP/REST API requests.

### Q: Can I burst above the limit temporarily?
**A:** No, the rate limiter uses a fixed window algorithm. Once you hit the limit, you must wait until the window resets (indicated by `X-RateLimit-Reset` header).

## Troubleshooting

### Issue: Getting 429 errors unexpectedly

**Possible causes:**
1. Shared IP address (multiple users behind NAT)
2. Aggressive polling without delays
3. Not implementing exponential backoff
4. Requests in tight loops

**Solutions:**
- Monitor `X-RateLimit-Remaining` header
- Implement exponential backoff with jitter
- Add delays between requests
- Use webhooks/subscriptions instead of polling (when available)
- Consider request batching

### Issue: Rate limit headers missing

**Possible causes:**
- Using a reverse proxy that strips headers
- Caching layer intercepting responses

**Solutions:**
- Check proxy configuration
- Ensure rate limit headers are preserved
- Make direct API requests to verify

### Issue: Rate limits too restrictive for use case

**Solutions:**
- Optimize request patterns (batch, cache, filter)
- Request enterprise tier access
- Use authenticated endpoints for higher limits

## Related Documentation

- [Error Codes Reference](./ERROR_CODES.md)
- [API Advanced Features](./API_ADVANCED_FEATURES.md) - Batch operations
- [Security Best Practices](./SECURITY.md) - API key management

## Support

For rate limit increases or issues:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Tag issues with: `api`, `rate-limiting`
