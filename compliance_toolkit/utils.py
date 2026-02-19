import os
import json
from typing import List, Dict, Any


def load_checklists(path: str) -> List[Dict[str, Any]]:
    checklists = []
    if not os.path.isdir(path):
        return checklists
    for fname in os.listdir(path):
        if not fname.endswith('.json'):
            continue
        full = os.path.join(path, fname)
        try:
            with open(full, 'r', encoding='utf-8') as f:
                items = json.load(f)
                checklists.extend(items)
        except Exception:
            continue
    return checklists


def scan_repo_for_keywords(repo_path: str, keywords: List[str]) -> bool:
    """Return True if any keyword appears in the repo files (simple heuristic)."""
    low_kw = [k.lower() for k in keywords]
    for root, _, files in os.walk(repo_path):
        for fn in files:
            if fn.endswith(('.png', '.jpg', '.jpeg', '.gif', '.class')):
                continue
            try:
                p = os.path.join(root, fn)
                with open(p, 'r', encoding='utf-8', errors='ignore') as f:
                    data = f.read().lower()
                    for k in low_kw:
                        if k in data:
                            return True
            except Exception:
                continue
    return False


def audit_repo(repo_path: str, checklists: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    results = []
    for item in checklists:
        present = scan_repo_for_keywords(repo_path, item.get('keywords', []))
        results.append({
            'id': item.get('id'),
            'title': item.get('title'),
            'description': item.get('description'),
            'present': present,
            'remediation': item.get('remediation'),
        })
    return results


def generate_markdown_report(results: List[Dict[str, Any]], title: str = 'Compliance Report') -> str:
    lines = [f'# {title}', '']
    passed = sum(1 for r in results if r['present'])
    lines.append(f'- Total checks: {len(results)}')
    lines.append(f'- Checks detected (heuristic pass): {passed}')
    lines.append('')
    for r in results:
        status = 'PASS' if r['present'] else 'FAIL'
        lines.append(f'## {r["id"]} - {r["title"]}  ')
        lines.append(f'- Status: **{status}**')
        lines.append(f'- Description: {r.get("description","")}')
        if not r['present']:
            lines.append(f'- Remediation: {r.get("remediation","")}')
        lines.append('')
    return '\n'.join(lines)
