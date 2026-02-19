#!/usr/bin/env python3
"""CLI entry for compliance toolkit."""
import argparse
import json
import os
import sys
from datetime import datetime
from zipfile import ZipFile

from .utils import load_checklists, audit_repo, generate_markdown_report


def cmd_audit(args):
    repo = args.repo or os.getcwd()
    checklist_dir = os.path.join(os.path.dirname(__file__), 'checklists')
    checks = load_checklists(checklist_dir)
    results = audit_repo(repo, checks)
    out = args.out
    if out:
        with open(out, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2)
        print(f'Wrote audit JSON to {out}')
    else:
        print(json.dumps(results, indent=2))


def cmd_report(args):
    repo = args.repo or os.getcwd()
    checklist_dir = os.path.join(os.path.dirname(__file__), 'checklists')
    checks = load_checklists(checklist_dir)
    results = audit_repo(repo, checks)
    md = generate_markdown_report(results, title=f'Compliance Report ({datetime.utcnow().isoformat()}Z)')
    out = args.out or 'compliance-report.md'
    with open(out, 'w', encoding='utf-8') as f:
        f.write(md)
    print(f'Wrote report to {out}')


def cmd_certify(args):
    repo = args.repo or os.getcwd()
    checklist_dir = os.path.join(os.path.dirname(__file__), 'checklists')
    checks = load_checklists(checklist_dir)
    results = audit_repo(repo, checks)
    md = generate_markdown_report(results, title=f'Certification Packet ({datetime.utcnow().isoformat()}Z)')
    out = args.out or 'certification-packet.zip'
    # create zip: report + raw audit JSON
    tmp_md = 'tmp_compliance_report.md'
    tmp_json = 'tmp_audit.json'
    with open(tmp_md, 'w', encoding='utf-8') as f:
        f.write(md)
    with open(tmp_json, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2)
    with ZipFile(out, 'w') as z:
        z.write(tmp_md)
        z.write(tmp_json)
    os.remove(tmp_md)
    os.remove(tmp_json)
    print(f'Wrote certification packet to {out}')


def cmd_checklist(args):
    checklist_dir = os.path.join(os.path.dirname(__file__), 'checklists')
    checks = load_checklists(checklist_dir)
    print(json.dumps(checks, indent=2))


def main(argv=None):
    parser = argparse.ArgumentParser(description='Contract Compliance Toolkit')
    sub = parser.add_subparsers(dest='cmd')
    pa = sub.add_parser('audit')
    pa.add_argument('--repo', help='Path to repository to audit')
    pa.add_argument('--out', help='Write audit JSON to file')
    pr = sub.add_parser('report')
    pr.add_argument('--repo', help='Path to repository to audit')
    pr.add_argument('--out', help='Output markdown report file')
    pc = sub.add_parser('certify')
    pc.add_argument('--repo', help='Path to repository to audit')
    pc.add_argument('--out', help='Output zip file for certification packet')
    pc = sub.add_parser('checklist')

    args = parser.parse_args(argv)
    if args.cmd == 'audit':
        cmd_audit(args)
    elif args.cmd == 'report':
        cmd_report(args)
    elif args.cmd == 'certify':
        cmd_certify(args)
    elif args.cmd == 'checklist':
        cmd_checklist(args)
    else:
        parser.print_help()


if __name__ == '__main__':
    main()
