#!/usr/bin/env python3
"""Check project-authored Markdown language counterparts and reciprocal links."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
INLINE_TEMPLATES = {'.github/ISSUE_TEMPLATE/bug_report.md', '.github/ISSUE_TEMPLATE/feature_request.md', '.github/PULL_REQUEST_TEMPLATE.md'}
ORIGINAL_TEXT = {'CLAUDE.md', 'assets/fonts/LICENSE.md'}

# These locally authored vendor patch notes preserve original upstream materials.
VENDOR_PAIRS = (
    ('vendor/alacritty_terminal/UPSTREAM.zh.md', 'vendor/alacritty_terminal/UPSTREAM.md'),
    ('vendor/vte/UPSTREAM.zh.md', 'vendor/vte/UPSTREAM.md'),
    ('vendor/egui_term/UPSTREAM.zh.md', 'vendor/egui_term/UPSTREAM.en.md'),
    ('vendor/egui-winit/UPSTREAM.zh.md', 'vendor/egui-winit/UPSTREAM.md'),
    ('vendor/egui_glow/UPSTREAM.zh.md', 'vendor/egui_glow/UPSTREAM.md'),
    ('vendor/winit/UPSTREAM.zh.md', 'vendor/winit/UPSTREAM.md'),
)


def check(paths, root=ROOT):
    errors = []
    files = {name for name in paths if (root / name).is_file()}
    checked = set()

    def pair(name, other):
        if other not in files or not (root / other).is_file():
            errors.append(f'missing bilingual counterpart: {name} -> {other}')
            return
        content = (root / name).read_text(encoding='utf-8-sig')
        if f']({Path(other).name})' not in content:
            errors.append(f'missing reciprocal language link: {name} -> {other}')
        checked.add(tuple(sorted((name, other))))

    for name in sorted(files):
        if not name.endswith('.md') or name in ORIGINAL_TEXT:
            continue  # tool entry or preserved third-party license
        if name in INLINE_TEMPLATES:
            continue  # the known issue and PR templates are bilingual on one page
        if name.startswith('vendor/'):
            continue  # upstream docs remain intact; local patch pairs are checked below
        other = (name[:-len('.en.md')] + '.md' if name.endswith('.en.md')
                 else name[:-len('.md')] + '.en.md')
        pair(name, other)

    for zh, en in VENDOR_PAIRS:
        for name, other in ((zh, en), (en, zh)):
            if name not in files or not (root / name).is_file():
                errors.append(f'missing vendor translation: {name}')
            else:
                pair(name, other)
    return errors, checked


def candidate_paths(root=ROOT):
    result = subprocess.run(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'],
        cwd=root, capture_output=True, check=True,
    )
    return set(result.stdout.decode('utf-8').split('\0')) - {''}


def digests(pairs, root=ROOT):
    # Compare reviewed UTF-8 text, independent of checkout/editor EOL and BOM.
    # Text I/O normalizes CRLF/CR to LF; genuine content edits still change SHA-256.
    return {name: hashlib.sha256((root / name).read_text(encoding='utf-8-sig').encode('utf-8')).hexdigest()
            for name in sorted({name for pair in pairs for name in pair})}


def verify_snapshots(pairs, root=ROOT):
    manifest = root / 'scripts/bilingual-manifest.json'
    if not manifest.is_file():
        return ['missing bilingual digest manifest; review translations and run --record']
    try:
        recorded = json.loads(manifest.read_text(encoding='utf-8'))
    except (ValueError, OSError) as exc:
        return [f'invalid bilingual digest manifest: {exc}']
    current = digests(pairs, root)
    return [f'bilingual content changed or absent in manifest: {name}'
            for name in sorted(set(recorded) | set(current))
            if recorded.get(name) != current.get(name)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--record', action='store_true', help='record current paired document hashes after bilingual review')
    args = parser.parse_args()
    errors, pairs = check(candidate_paths())
    if errors:
        for error in errors:
            print(error)
        return 1
    if args.record:
        manifest = ROOT / 'scripts/bilingual-manifest.json'
        manifest.write_text(json.dumps(digests(pairs), indent=2, ensure_ascii=False) + '\n', encoding='utf-8', newline='\n')
        print(f'Recorded {len(pairs)} bilingual pairs; semantic equivalence still requires review.')
        return 0
    errors = verify_snapshots(pairs)
    for error in errors:
        print(error)
    if errors:
        print('Review both languages and then run: python scripts/check_bilingual.py --record')
        return 1
    print(f'Verified {len(pairs)} bilingual pairs.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
