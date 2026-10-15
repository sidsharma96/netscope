"""Shared Markdown helpers for the documentation checks.

The book is rendered by mdBook, so heading anchors follow mdBook's rule:
take the heading's visible text, lowercase it, keep letters, digits, '-' and
'_', turn whitespace into '-', and drop everything else. A repeated anchor in
one page gets a numeric suffix ('-1', '-2', ...). GitHub uses the same rule
for the characters this book uses.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
REPOSITORY = 'https://github.com/sidsharma96/netscope'

FENCE = re.compile(r'^(```|~~~)')
LINK = re.compile(r'(!?)\[((?:[^\[\]]|\[[^\]]*\])*)\]\(([^)\s]+)\)')


def without_code(text):
    """Return the text with fenced blocks blanked and inline code removed."""
    out, fenced = [], False
    for line in text.splitlines():
        if FENCE.match(line.strip()):
            fenced = not fenced
            out.append('')
            continue
        out.append('' if fenced else re.sub(r'`[^`]*`', '', line))
    return '\n'.join(out)


def heading_lines(text):
    """Yield (level, raw title) for ATX headings outside fenced code."""
    fenced = False
    for line in text.splitlines():
        if FENCE.match(line.strip()):
            fenced = not fenced
            continue
        match = None if fenced else re.match(r'^(#{1,6})\s+(.*?)\s*#*\s*$', line)
        if match:
            yield len(match[1]), match[2]


def visible(title):
    title = re.sub(r'!?\[([^\]]*)\]\([^)]*\)', r'\1', title)
    title = re.sub(r'<[^>]+>', '', title)
    title = title.replace('`', '')
    title = re.sub(r'(\*\*|__|\*)', '', title)
    return title


def slug(title):
    text = visible(title).strip().lower()
    return ''.join('-' if ch.isspace() else ch for ch in text if ch.isalnum() or ch in '-_' or ch.isspace())


def anchors(text):
    seen, result = {}, set()
    for _, title in heading_lines(text):
        base = slug(title)
        count = seen.get(base, 0)
        result.add(base if count == 0 else f'{base}-{count}')
        seen[base] = count + 1
    return result


def links(text):
    """Yield (is_image, label, href) for Markdown links outside code."""
    for match in LINK.finditer(without_code(text)):
        yield bool(match[1]), match[2], match[3]


def summary_pages():
    """Return the Markdown pages listed in docs/SUMMARY.md, in order."""
    text = (DOCS / 'SUMMARY.md').read_text()
    return [DOCS / href for _, _, href in links(text) if href.endswith('.md')]
