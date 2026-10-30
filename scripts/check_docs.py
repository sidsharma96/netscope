"""Check the Markdown book: navigation, links, anchors, code links and questions.

The book is rendered by mdBook from docs/SUMMARY.md. These checks run on the
Markdown sources, so they need neither mdBook nor a network connection.

    python3 scripts/check_docs.py              # check the sources
    python3 scripts/check_docs.py --site DIR   # also check a built site
"""
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from doclinks import DOCS, ROOT, REPOSITORY, anchors, links, summary_pages, without_code  # noqa: E402
import sync_questions  # noqa: E402
import check_excerpts  # noqa: E402

failures = []


def fail(message):
    failures.append(message)


def rel(path):
    return path.relative_to(ROOT)


# 1. Navigation: SUMMARY.md lists every Markdown page under docs/ exactly once.
pages = summary_pages()
listed = {page.resolve() for page in pages}
if len(listed) != len(pages):
    fail('SUMMARY.md lists a page more than once')
for page in pages:
    if not page.is_file():
        fail(f'SUMMARY.md lists a missing page: {rel(page)}')
sources = sorted(p for p in DOCS.rglob('*.md') if 'book' not in p.relative_to(DOCS).parts[:1] and p.name != 'SUMMARY.md')
for source in sources:
    if source.resolve() not in listed:
        fail(f'Page is not reachable from SUMMARY.md: {rel(source)}')
print(f'checked navigation: {len(pages)} pages in SUMMARY.md')

# 2. Relative links and heading anchors resolve inside the book.
# mdBook renders docs/README.md as index.html, but rewrites links to it as
# README.html, so pages must not link to it. Links that leave docs/ break on
# the hosted site and must be absolute GitHub links instead.
INDEX = (DOCS / 'README.md').resolve()
SITE_SUFFIXES = {'.md', '.png', '.svg', '.jpg', '.jpeg', '.gif', '.webp', '.csv'}
anchor_cache = {}


def page_anchors(path):
    if path not in anchor_cache:
        anchor_cache[path] = anchors(path.read_text())
    return anchor_cache[path]


relative_links = 0
for source in [DOCS / 'SUMMARY.md', *sources]:
    for _, _, href in links(source.read_text()):
        if '://' in href or href.startswith('mailto:'):
            continue
        relative_links += 1
        where = f'{rel(source)}: {href}'
        path, _, fragment = href.partition('#')
        target = (source.parent / path).resolve() if path else source.resolve()
        if not target.is_relative_to(DOCS.resolve()):
            fail(f'Relative link leaves docs/; use an absolute GitHub link: {where}')
            continue
        if not target.is_file():
            fail(f'Missing link target: {where}')
            continue
        if target == INDEX and source.name != 'SUMMARY.md':
            fail(f'Link to the book index (docs/README.md) breaks in mdBook: {where}')
        if target.suffix not in SITE_SUFFIXES:
            fail(f'Link to a file the site should not publish: {where}')
        if target.suffix == '.md' and target not in listed and source.name != 'SUMMARY.md':
            fail(f'Link to a page missing from SUMMARY.md: {where}')
        if fragment and target.suffix == '.md' and fragment not in page_anchors(target):
            fail(f'Missing heading anchor: {where}')
print(f'checked {relative_links} relative links and anchors')

# 3. Code links: absolute GitHub links on main. A link to source code needs a
# line anchor, and the symbol named in its text must appear near those lines.
GITHUB = re.compile(r'\[((?:[^\[\]]|\[[^\]]*\])*)\]\(' + re.escape(REPOSITORY) + r'/(blob|tree)/main/([^)#\s]+)(?:#L(\d+)(?:-L(\d+))?)?\)')
CODE_SUFFIXES = {'.rs', '.py', '.toml', '.mjs', '.js'}
SLACK = 3
code_links = 0
for source in [ROOT / 'README.md', ROOT / 'AGENTS.md', *sources]:
    text = source.read_text()
    for _, _, href in links(text):
        if '://' not in href and not href.startswith('#'):
            if Path(href.partition('#')[0]).suffix in CODE_SUFFIXES:
                fail(f'Use an absolute GitHub link for code: {rel(source)}: {href}')
    for label, kind, path, first, last in GITHUB.findall(text):
        where = f'{rel(source)}: {path}' + (f'#L{first}' if first else '')
        target = ROOT / path
        if kind == 'tree':
            if not target.is_dir():
                fail(f'Missing linked directory: {where}')
            continue
        if not target.is_file():
            fail(f'Missing linked file: {where}')
            continue
        if target.suffix not in CODE_SUFFIXES:
            continue
        if not first:
            fail(f'Code link needs a line anchor: {where}')
            continue
        symbols = re.findall(r'`([^`]+)`', label)
        if not symbols:
            fail(f'Code link text must name the symbol in backticks: {where}: {label}')
            continue
        lines = target.read_text().splitlines()
        start, end = int(first), int(last or first)
        if not 1 <= start <= end <= len(lines):
            fail(f'Line anchor outside file: {where}')
            continue
        nearby = '\n'.join(lines[max(0, start - 1 - SLACK):end + SLACK])
        symbol = symbols[0].split('::')[-1].removesuffix('()')
        # A whole-file reference (for example a script with no functions) names the file.
        if symbol != target.name and symbol not in nearby:
            fail(f'Symbol {symbol!r} not near anchored lines: {where}')
        code_links += 1
print(f'checked {code_links} code links against their anchored lines')

# 4. Question bank: every question uses the shared format, its level matches
# its page, its tags name teaching chapters, and its links work from both the
# bank and the chapters that copy it.
counts = {}
for level in sync_questions.LEVELS:
    page = DOCS / 'questions' / f'{level}.md'
    entries = sync_questions.questions(level)
    counts[level] = len(entries)
    for title, tags, block in entries:
        where = f'{rel(page)}: {title}'
        if f'**Level:** {level.capitalize()}' not in block:
            fail(f'Question level does not match its page: {where}')
        for part in ['<details>', '<summary>Answer</summary>', '**Learn more:**', '</details>']:
            if part not in block:
                fail(f'Question is missing {part!r}: {where}')
        if not tags:
            fail(f'Question has no chapters tag: {where}')
        for tag in tags:
            if tag not in sync_questions.TEACHING:
                fail(f'Question tags an unknown or non-teaching chapter {tag!r}: {where}')
        exercises = sync_questions.exercise_tags(block)
        if len(exercises) != len(set(exercises)):
            fail(f'Duplicate exercise chapter: {where}')
        for tag in exercises:
            if tag not in tags:
                fail(f'Exercise chapter must also be a topic tag: {where}: {tag}')
        learn = block.split('**Learn more:**')[-1]
        if not any(not href.startswith('http') for _, _, href in links(learn)):
            fail(f'Learn more needs a link to the teaching section: {where}')
        for _, _, href in links(block):
            if '://' not in href and not href.startswith('../'):
                fail(f'Question links must start with ../ so chapter copies resolve: {where}: {href}')
        if re.search(r'^```', block, re.MULTILINE):
            fail(f'Question bank must not use fenced code (chapter examples are compiled): {where}')
print('checked question bank: ' + ', '.join(f'{n} {level}' for level, n in counts.items()))

# 5. Each teaching chapter ends with an up-to-date "Check your understanding".
bank = sync_questions.bank()
for slug in sync_questions.TEACHING:
    path = DOCS / 'chapters' / f'{slug}.md'
    if path.read_text() != sync_questions.expected(path, bank):
        fail(f'Check-your-understanding section is stale; run scripts/sync_questions.py: {rel(path)}')
    chosen = len(sync_questions.selection(slug, bank))
    if chosen > sync_questions.MAX_QUESTIONS:
        fail(f'{rel(path)} has too many selected exercises; review its focus')
    if chosen < sync_questions.MIN_QUESTIONS:
        fail(f'{rel(path)} has {chosen} tagged questions; it needs {sync_questions.MIN_QUESTIONS}')
print(f'checked {len(sync_questions.TEACHING)} chapter question sections')

# 6. Pages written for the hosted book follow the prose rule against em dashes.
for page in [DOCS / 'README.md', DOCS / 'GLOSSARY.md', DOCS / 'contracts' / 'overview.md', *(DOCS / 'questions').glob('*.md')]:
    if page.is_file() and '—' in without_code(page.read_text()):
        fail(f'Em dash in prose: {rel(page)}')

# 7. Optionally, a built site holds only documentation and its assets.
if '--site' in sys.argv:
    site = Path(sys.argv[sys.argv.index('--site') + 1])
    allowed = {'.html', '.css', '.js', '.json', '.woff', '.woff2', '.ttf', '.eot', '.otf',
               '.svg', '.png', '.jpg', '.jpeg', '.gif', '.webp', '.ico', '.csv', '.txt', ''}
    published = sorted(p for p in site.rglob('*') if p.is_file())
    for path in published:
        if path.suffix.lower() not in allowed or path.name.startswith('.') and path.name != '.nojekyll':
            fail(f'Built site contains a non-documentation file: {path.relative_to(site)}')
    for page in pages:
        out = site / page.relative_to(DOCS).with_suffix('.html')
        if page.resolve() == INDEX:
            out = site / 'index.html'
        if not out.is_file():
            fail(f'Built site is missing {out.relative_to(site)}')
    print(f'checked built site: {len(published)} files')

excerpt_count, excerpt_errors = check_excerpts.check_book(ROOT, sources)
failures.extend(excerpt_errors)
print(f'checked {excerpt_count} current and historical source excerpts')

if failures:
    print('\n'.join(f'FAIL: {message}' for message in failures))
    sys.exit(1)
print('PASS: documentation checks')
