#!/bin/bash
curl -X POST http://localhost:3001/api/vulnerabilities/sync \
-H "Content-Type: application/json" \
-d '[
  {
    "cve_id": "CVE-2026-0001",
    "description": "Mock high severity vulnerability in dependency foo",
    "severity": "High",
    "package_name": "foo-dependency",
    "patched_versions": ["1.2.3"]
  }
]'

# Run the CLI to trigger a scan
echo "Testing foo-dependency@1.0.0 (vulnerable)"
# Not possible since cargo isn't installed.
