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
        assert (DOCS / href).resolve().exists(), f'Missing local link: {href}'
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
