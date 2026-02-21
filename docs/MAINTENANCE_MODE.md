# Contract Maintenance Mode

This feature allows contract publishers to put their contracts into maintenance mode, preventing write operations while displaying a custom message to users.

## Features

- **Read-only mode**: Write operations return 503 during maintenance
- **Custom messages**: Publishers can set informative messages for users
- **Scheduled end**: Automatically exit maintenance at a specified time
- **History tracking**: All maintenance windows are logged
- **API status checks**: Users can query maintenance status programmatically

## Database Schema

The feature adds:
- `maintenance_windows` table to track all maintenance periods
- `is_maintenance` boolean flag on contracts table
- Automatic scheduler to end maintenance at scheduled times

## API Endpoints

### Start Maintenance
```bash
POST /api/contracts/{id}/maintenance
Content-Type: application/json

{
  "message": "Upgrading to v2.0 - back online at 3pm UTC",
  "scheduled_end_at": "2026-02-20T15:00:00Z"  // optional
}
```

### End Maintenance
```bash
DELETE /api/contracts/{id}/maintenance
```

### Check Status
```bash
GET /api/contracts/{id}/maintenance
```

Response:
```json
{
  "is_maintenance": true,
  "current_window": {
    "id": "...",
    "contract_id": "...",
    "message": "Upgrading to v2.0",
    "started_at": "2026-02-20T10:00:00Z",
    "scheduled_end_at": "2026-02-20T15:00:00Z",
    "ended_at": null,
    "created_by": "...",
    "created_at": "2026-02-20T10:00:00Z"
  }
}
```

### Get History
```bash
GET /api/contracts/{id}/maintenance/history
```

## Middleware Behavior

The maintenance middleware intercepts write operations (POST, PUT, PATCH, DELETE) to contracts in maintenance mode and returns:

```json
HTTP 503 Service Unavailable
{
  "error": "maintenance_mode",
  "message": "Upgrading to v2.0 - back online at 3pm UTC"
}
```

Read operations (GET) continue to work normally.

## Frontend Integration

The frontend displays a yellow banner on contract pages when maintenance is active:

```tsx
import { maintenanceApi } from '@/lib/api';

// Check status
const status = await maintenanceApi.getStatus(contractId);

// Start maintenance
await maintenanceApi.start(contractId, "System upgrade in progress", "2026-02-20T15:00:00Z");

// End maintenance
await maintenanceApi.end(contractId);
```

## Background Scheduler

A background task runs every 60 seconds to automatically end maintenance windows that have reached their `scheduled_end_at` time.

## Migration

Run the migration to add the required tables:

```bash
sqlx migrate run --source database/migrations
```

The migration file is `004_maintenance_mode.sql`.
