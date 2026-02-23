# Contract Maturity Levels

## Overview
Maturity levels provide clear expectations about contract stability and production-readiness.

## Maturity Levels

### Alpha (Experimental)
- **Purpose**: Early development, experimental features
- **Requirements**: None (default for new contracts)
- **Badge Color**: Purple
- **Use Case**: Testing new concepts, proof-of-concepts

### Beta (Testing)
- **Purpose**: Feature-complete but needs testing
- **Requirements**:
  - ✅ Source code verified
  - ✅ At least 1 version published
- **Badge Color**: Blue
- **Use Case**: Pre-production testing, gathering feedback

### Stable (Production Ready)
- **Purpose**: Ready for production use
- **Requirements**:
  - ✅ Source code verified
  - ✅ At least 2 versions published
  - ✅ At least 10 contract interactions
- **Badge Color**: Green
- **Use Case**: Production deployments

### Mature (Battle-tested)
- **Purpose**: Proven reliability over time
- **Requirements**:
  - ✅ Source code verified
  - ✅ At least 5 versions published
  - ✅ At least 100 contract interactions
- **Badge Color**: Emerald
- **Use Case**: Mission-critical applications

### Legacy (Deprecated)
- **Purpose**: Deprecated, migration recommended
- **Requirements**: Manual assignment by publisher
- **Badge Color**: Gray
- **Use Case**: Contracts being phased out

## API Endpoints

### Update Maturity Level
```bash
PUT /api/contracts/{id}/maturity
Content-Type: application/json

{
  "maturity": "stable",
  "reason": "Passed all production tests"
}
```

### Check Requirements
```bash
GET /api/contracts/{id}/maturity/requirements
```

Response:
```json
[
  {
    "level": "beta",
    "met": true,
    "criteria": [
      {
        "name": "verified",
        "required": true,
        "met": true,
        "description": "Contract source code must be verified"
      },
      {
        "name": "versions",
        "required": true,
        "met": true,
        "description": "At least 1 version published"
      }
    ]
  },
  {
    "level": "stable",
    "met": false,
    "criteria": [...]
  }
]
```

### Get Maturity History
```bash
GET /api/contracts/{id}/maturity/history
```

Response:
```json
[
  {
    "id": "...",
    "contract_id": "...",
    "from_level": "beta",
    "to_level": "stable",
    "reason": "Passed all production tests",
    "changed_by": "...",
    "changed_at": "2026-02-20T10:00:00Z"
  }
]
```

### Filter by Maturity
```bash
GET /api/contracts?maturity=stable
```

## Frontend Integration

### Display Badge
```tsx
import MaturityBadge from '@/components/MaturityBadge';

<MaturityBadge level={contract.maturity} size="md" />
```

### Update Maturity
```tsx
import { maturityApi } from '@/lib/api';

await maturityApi.update(contractId, 'stable', 'Production ready');
```

### Check Requirements
```tsx
const requirements = await maturityApi.checkRequirements(contractId);
const canUpgrade = requirements.find(r => r.level === 'stable')?.met;
```

## Database Schema

```sql
-- Maturity enum
CREATE TYPE maturity_level AS ENUM ('alpha', 'beta', 'stable', 'mature', 'legacy');

-- Contract field
ALTER TABLE contracts ADD COLUMN maturity maturity_level NOT NULL DEFAULT 'alpha';

-- Change tracking
CREATE TABLE maturity_changes (
    id UUID PRIMARY KEY,
    contract_id UUID REFERENCES contracts(id),
    from_level maturity_level,
    to_level maturity_level NOT NULL,
    reason TEXT,
    changed_by UUID REFERENCES publishers(id),
    changed_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Graduation Process

1. **Check Requirements**: Call `/maturity/requirements` endpoint
2. **Verify Criteria**: Ensure all required criteria are met
3. **Update Level**: Call PUT `/maturity` with new level and reason
4. **Logged**: Change is automatically logged in `maturity_changes` table

## Compliance

All maturity level changes are logged with:
- Previous level
- New level
- Reason for change
- Who made the change
- Timestamp

This provides a complete audit trail for compliance and governance.

## Best Practices

1. **Start Alpha**: All new contracts default to alpha
2. **Verify First**: Get source verified before moving to beta
3. **Test Thoroughly**: Gather real usage data before stable
4. **Document Changes**: Always provide a reason when changing levels
5. **Deprecate Gracefully**: Use legacy level when phasing out contracts
