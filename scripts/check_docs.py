"""Check generated documentation structure, local links, and reproducibility."""
from pathlib import Path
from html.parser import HTMLParser
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
class Reader(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.links, self.pages = [], [], []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs: self.ids.append(attrs['id'])
        if tag == 'a' and 'href' in attrs: self.links.append(attrs['href'])
        if 'data-page' in attrs: self.pages.append(attrs['data-page'])

before = (DOCS / 'index.html').read_bytes()
subprocess.run([sys.executable, str(ROOT / 'scripts/build_docs.py')], check=True)
assert before == (DOCS / 'index.html').read_bytes(), 'Reader differs from Markdown sources'
reader = Reader()
reader.feed(before.decode())
assert len(reader.ids) == len(set(reader.ids)), 'Duplicate HTML IDs'
chapters = json.loads((DOCS / 'chapters.json').read_text())
assert reader.pages == [chapter['slug'] for chapter in chapters]
for href in reader.links:
    if href.startswith('#'):
        slug, _, section = href[1:].partition('/')
        if slug == 'reading': continue
        assert slug in reader.pages, f'Unknown chapter: {href}'
        if section: assert f'{slug}--{section}' in reader.ids, f'Unknown section: {href}'
    elif '://' not in href:
        path, _, fragment = href.partition('#')
        target = (DOCS / path).resolve()
        assert target.is_relative_to(ROOT), f'Link leaves repository: {href}'
        assert target.is_file(), f'Missing local link: {href}'
        if fragment and target.suffix == '.md':
            headings = re.findall(r'^#+ (.+)$', target.read_text(), re.MULTILINE)
            anchors = {re.sub(r'[^a-z0-9]+', '-', h.lower()).strip('-') for h in headings}
            assert fragment in anchors, f'Missing Markdown heading: {href}'
for path in ['assets/book.css', 'assets/book.js']:
    assert (DOCS / path).is_file()
print(f'PASS: reproducible reader, {len(chapters)} chapters, unique anchors, and local links')

# Check source links independently: the generated reader has a different base path.
for source in [DOCS / 'README.md', *(DOCS / 'chapters').glob('*.md')]:
    for href in re.findall(r'\[[^\]]+\]\(([^)]+)\)', source.read_text()):
        if '://' in href:
            continue
        path, _, fragment = href.partition('#')
        target = (source.parent / path).resolve() if path else source
        assert target.is_relative_to(ROOT), f'Link leaves repository: {source}: {href}'
        assert target.is_file(), f'Missing source link: {source}: {href}'
        if fragment and target.suffix == '.md':
            headings = re.findall(r'^#+ (.+)$', target.read_text(), re.MULTILINE)
            anchors = {re.sub(r'[^a-z0-9]+', '-', h.lower()).strip('-') for h in headings}
            assert fragment in anchors, f'Missing Markdown heading: {href}'
print('PASS: Markdown chapter links resolve within the repository')

# Code links must be absolute GitHub links on main with a line anchor, and the
# symbol named in the link text must still appear near the anchored lines.
GITHUB = re.compile(r'\[([^\]]+)\]\(https://github\.com/sidsharma96/netscope/blob/main/([^)#\s]+)(?:#L(\d+)(?:-L(\d+))?)?\)')
CODE_SUFFIXES = {'.rs', '.py', '.toml', '.mjs', '.js'}
SLACK = 3
code_links = 0
for source in [ROOT / 'README.md', *sorted(DOCS.rglob('*.md'))]:
    text = source.read_text()
    for href in re.findall(r'\]\(([^)\s]+)\)', text):
        if '://' not in href and not href.startswith('#'):
            assert Path(href.partition('#')[0]).suffix not in CODE_SUFFIXES, \
                f'Use an absolute GitHub link for code: {source.relative_to(ROOT)}: {href}'
    for label, path, first, last in GITHUB.findall(text):
        where = f'{source.relative_to(ROOT)}: {path}#L{first}'
        target = ROOT / path
        assert target.is_file(), f'Missing linked file: {where}'
        assert first, f'Code link needs a line anchor: {where}'
        symbols = re.findall(r'`([^`]+)`', label)
        assert symbols, f'Code link text must name the symbol in backticks: {where}: {label}'
        lines = target.read_text().splitlines()
        start, end = int(first), int(last or first)
        assert 1 <= start <= end <= len(lines), f'Line anchor outside file: {where}'
        nearby = '\n'.join(lines[max(0, start - 1 - SLACK):end + SLACK])
        symbol = symbols[0].split('::')[-1].removesuffix('()')
        # A whole-file reference (for example a script with no functions) names the file.
        if symbol != target.name:
            assert symbol in nearby, f'Symbol {symbol!r} not near anchored lines: {where}'
        code_links += 1
print(f'PASS: {code_links} code links name a symbol at their anchored lines')
