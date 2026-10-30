"""Verify current excerpts against local source and historical excerpts against Git.

Run directly, or through check_docs.py, only in a checkout with the private book.
A marker ends in @current or a recorded Git revision. Historical code is allowed
only for a bug or performance lesson, declared immediately above its marker.
CI tests use temporary files.
"""
from pathlib import Path
from functools import lru_cache
import re
import subprocess

MARKER = re.compile(r"<!-- excerpt: (\S+?)#L(\d+)-L(\d+) @(current|[0-9a-f]{7,40}) -->")
HISTORICAL = re.compile(r"<!-- historical: (bug|performance); \S.* -->")
CUT = re.compile(r"^\s*(//|#|--)\s*….*$")

@lru_cache(maxsize=None)
def historical_lines(repo, revision, path):
    result = subprocess.run(['git', '-C', str(repo), 'show', f'{revision}:{path}'],
                            capture_output=True, text=True)
    return result.stdout.splitlines() if result.returncode == 0 else None


def check(repo, markdown):
    errors = []
    lines = markdown.read_text().splitlines()
    for i, line in enumerate(lines):
        if not line.startswith('<!-- excerpt:'):
            continue
        marker = MARKER.fullmatch(line)
        where = f'{markdown}:{i + 1}'
        if not marker:
            errors.append(f'{where}: malformed excerpt marker')
            continue
        path, first, last, revision = marker.groups()
        first, last = int(first), int(last)
        if revision != 'current' and (i == 0 or not HISTORICAL.fullmatch(lines[i - 1])):
            errors.append(f'{where}: historical code needs a bug or performance reason immediately above its marker')
        if revision == 'current' and i > 0 and lines[i - 1].startswith('<!-- historical:'):
            errors.append(f'{where}: current code must not have a historical label')
        if i + 1 >= len(lines) or not lines[i + 1].startswith('```'):
            errors.append(f'{where}: marker must precede a code fence')
            continue
        end = next((j for j in range(i + 2, len(lines)) if lines[j].startswith('```')), None)
        if end is None:
            errors.append(f'{where}: excerpt fence is not closed')
            continue
        if revision == 'current':
            source = (repo / path).resolve()
            src = source.read_text().splitlines() if source.is_relative_to(repo.resolve()) and source.is_file() else None
        else:
            src = historical_lines(repo, revision, path)
        if src is None:
            errors.append(f'{where}: cannot read {path} at {revision}')
        elif not 1 <= first <= last <= len(src):
            errors.append(f'{where}: lines outside {path}')
        else:
            problem = align([s.strip() for s in src[first - 1:last]], lines[i + 2:end])
            if problem:
                errors.append(f'{where}: {path}: {problem}')
    return errors


def align(window, body):
    """Match the block's lines to the source in order, choosing the match that skips the fewest
    source lines without a cut comment. Repeated lines such as "}" make a greedy match unreliable.
    Doc comments (/// and //!) may be left out silently."""
    real = lambda s: bool(s) and not s.startswith(("///", "//!"))
    before = [0]
    for s in window:
        before.append(before[-1] + real(s))
    items, cut = [], False
    for b in body:
        if CUT.match(b):
            cut = True
        elif b.strip():
            items.append((b.strip(), cut))
            cut = False
    reached = {-1: (0, None)}  # source index of the last matched line -> (silent skips, an example)
    for want, after_cut in items:
        nxt = {}
        for k, s in enumerate(window):
            if s != want:
                continue
            best = None
            for prev, (cost, example) in reached.items():
                if prev >= k:
                    continue
                skipped = 0 if after_cut or prev == -1 else before[k] - before[prev + 1]
                if skipped:
                    example = next(s for s in window[prev + 1:k] if real(s))
                if best is None or cost + skipped < best[0]:
                    best = (cost + skipped, example)
            if best:
                nxt[k] = best
        if not nxt:
            return f"line not found in order: {want}"
        reached = nxt
    cost, example = min(reached.values(), key=lambda v: v[0])
    if cost:
        return f"drops {cost} source line(s) without a cut comment, starting near: {example}"
    return None


def check_book(repo, pages):
    failures = [error for page in pages for error in check(repo, page)]
    count = sum(bool(MARKER.fullmatch(line)) for page in pages for line in page.read_text().splitlines())
    return count, failures


if __name__ == '__main__':
    import sys
    root = Path(__file__).resolve().parents[1]
    pages = [Path(p) for p in sys.argv[1:]] or [p for p in (root / 'docs').rglob('*.md') if 'book' not in p.parts]
    count, errors = check_book(root, pages)
    print('\n'.join(errors) if errors else f'PASS: {count} current and historical excerpts')
    sys.exit(bool(errors))
