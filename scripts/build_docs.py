"""Build the offline reader from the project's small Markdown subset."""
from pathlib import Path
import html
import json
import re
import posixpath

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'


def reader_href(href):
    if '://' in href or href.startswith('#'):
        return href
    path, separator, section = href.partition('#')
    if path in {chapter['slug'] + '.md' for chapter in chapters}:
        return '#' + path[:-3] + ('/' + section if separator else '')
    return posixpath.normpath('chapters/' + path) + (separator + section)


def inline(text):
    text = html.escape(text)
    text = re.sub(r'`([^`]+)`', r'<code>\1</code>', text)
    return re.sub(r'\[([^\]]+)\]\(([^)]+)\)', lambda m: f'<a href="{reader_href(m[2])}">{m[1]}</a>', text)


def render(text):
    out, paragraph, code = [], [], None
    headings = []
    def flush():
        if paragraph:
            out.append('<p>' + inline(' '.join(paragraph)) + '</p>')
            paragraph.clear()
    for line in text.splitlines():
        if code is not None:
            if line.startswith('```'):
                label = {'sh': 'Terminal command', 'text': 'Illustrative diagram', 'compile_fail': 'Rust · expected compiler error'}.get(code[0], code[0] + ' · teaching example')
                out.append('<div class="code-block"><div class="code-label">' + html.escape(label) + '</div><pre><code>' + html.escape('\n'.join(code[1])) + '</code></pre></div>')
                code = None
            else:
                code[1].append(line)
        elif line.startswith('```'):
            flush()
            code = (line[3:] or 'text', [])
        elif line.startswith('#'):
            flush()
            level = len(line) - len(line.lstrip('#'))
            title = line[level:].strip()
            anchor = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')
            out.append(f'<h{level} id="{anchor}">{inline(title)}</h{level}>')
            if level == 2:
                headings.append((anchor, title))
        elif line == '<!-- redirect-evidence-demo -->':
            flush()
            out.append('<figure class="import-flow"><figcaption>FIG. 10.1 · One instruction, two possible follow-ups <em>Illustrative</em></figcaption><pre aria-label="Request zero ends at ten milliseconds; requests one and two are both possible follow-ups">#0  GET /old-books    start 0 ms · end 10 ms\n    302 · Location: /books\n             │\n             ├ · · possible · · → #1 GET /books · start 20 ms\n             │\n             └ · · possible · · → #2 GET /books · start 30 ms\n\nRecorded: status, Location, URLs, methods, times\nSuggested: either later request could be a follow-up</pre></figure>')
        elif line == '<!-- grouped-duration-demo -->':
            flush()
            out.append('<figure class="import-flow"><figcaption>FIG. 09.1 · Two requests overlap <em>Illustrative</em></figcaption><pre aria-label="Two simultaneous requests take 40 and 60 milliseconds; their durations sum to 100 milliseconds">Time          0          20         40         60 ms\nRequest A     ├──────────────────────┤          40 ms\nRequest B     ├─────────────────────────────────┤ 60 ms\n\nSum of reported durations: 40 + 60 = 100 ms\nInterval from first start to last finish: 60 ms</pre></figure>')
        elif line == '<!-- view-flow -->':
            flush()
            out.append('<figure class="import-flow"><figcaption>FIG. 08.1 · Keep the selection while the view changes</figcaption><pre aria-label="Navigation preserves the selected request">Request list                         Request details\nRequest 1                            Read request 2\n&gt; Request 2 ─────── Enter ──────────→ Scroll through its fields\nRequest 0\n         ←────────── Esc ────────────\nSame selection and list position\n\nThe view state changes. Both screens read the same stored capture.</pre></figure>')
        elif line == '<!-- import-flow -->':
            flush()
            out.append('<figure class="import-flow"><figcaption>FIG. 07.1 · Give recorded fields a meaning before using them</figcaption><pre aria-label="Reading, validating, and retaining a capture">Capture file\n     │ read the JSON structure\n     ↓\nNamed fields: text, numbers, and lists\n     │ check what each field means\n     ├── required field invalid → report a located error\n     ↓\nRequests retain values, absence, and body warnings\n     │ accept every entry\n     ↓\nA complete session is ready to inspect</pre></figure>')
        elif line == '<!-- ownership-demo -->':
            flush()
            out.append('''<figure class="ownership-demo"><figcaption><span>FIG. 04.1</span> One allocation, changing ownership <em>Illustrative</em></figcaption><div class="diagram"><div class="binding" id="owner-box"><small>OWNER</small><strong id="owner-name">url</strong><span>String</span><div class="fields">buffer location · bytes used · room available</div></div><div class="connector" aria-hidden="true">─────▶</div><div class="allocation"><small>ALLOCATED BYTES</small><code>https://example.test/api/books</code><span>One text buffer</span></div><div class="borrow-box" id="borrow-box" hidden><small>SHARED REFERENCE</small><strong>&amp;request_url</strong><span>draw_request reads the same bytes</span></div></div><div class="step-description" id="step-description" aria-live="polite"></div><div class="step-controls"><button id="step-prev" type="button">← Previous</button><span id="step-count"></span><button id="step-next" type="button">Next →</button></div></figure>''')
        elif line == '<!-- ownership-answer -->':
            flush()
            out.append('<details><summary>Show the exercise explanation</summary><p>Using <code>url</code> after the move fails because ownership transferred to <code>request_url</code>. A shared borrow lets you read the existing string while its owner remains valid. Calling <code>clone()</code> creates an independent string with its own buffer. That copy costs allocation and copying of the text bytes.</p></details>')
        elif not line.strip():
            flush()
        else:
            paragraph.append(line)
    flush()
    if code is not None:
        raise ValueError('Unclosed code block')
    return '\n'.join(out), headings


chapters = json.loads((DOCS / 'chapters.json').read_text())
nav, articles, outlines = [], [], []
for position, chapter in enumerate(chapters, start=1):
    slug, title = chapter['slug'], chapter['title']
    body, headings = render((DOCS / 'chapters' / f'{slug}.md').read_text())
    for anchor, _ in headings:
        body = body.replace(f'id="{anchor}"', f'id="{slug}--{anchor}"')
    for old, target in chapter.get('section_aliases', {}).items():
        if target not in {anchor for anchor, _ in headings}:
            raise ValueError(f'Unknown alias target: {slug}/{target}')
        marker = f'<h2 id="{slug}--{target}"'
        body = body.replace(marker, f'<span class="section-alias" id="{slug}--{old}" aria-hidden="true"></span>' + marker)
    number = f'{position:02d}'
    nav.append(f'<a class="chapter-link" href="#{slug}" data-chapter="{slug}"><span>{number}</span>{html.escape(title)}</a>')
    articles.append(f'<article data-page="{slug}" hidden><div class="eyebrow">/book/{slug[3:]} <span>CHAPTER {number}</span></div>{body}<footer class="chapter-footer">Netscope field notes · Source: chapters/{slug}.md</footer></article>')
    outlines.append(f'<div data-outline="{slug}" hidden>' + ''.join(f'<a href="#{slug}/{a}" data-section="{a}">{html.escape(t)}</a>' for a,t in headings) + '</div>')
page = '''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="color-scheme" content="dark light"><title>Netscope — Rust field notes</title><link rel="stylesheet" href="assets/book.css"><script src="assets/book.js" defer></script></head><body><a class="skip-link" href="#reading">Skip to reading</a><header class="topbar"><a class="brand" href="#01-start-here"><span class="brand-mark">⌁</span> netscope<span class="book-label">/ FIELD NOTES</span></a><div class="header-actions"><button id="menu-toggle" aria-expanded="false" aria-controls="chapter-nav">Chapters</button><label class="search"><span>⌕</span><input id="search" type="search" aria-label="Search chapters" placeholder="Find a chapter or concept"><kbd>/</kbd></label><button id="theme-toggle" type="button" aria-label="Switch to light theme">Light</button></div></header><div class="layout"><aside class="sidebar" id="chapter-nav"><div class="sidebar-heading">BUILDING NETSCOPE</div><nav aria-label="Chapters">NAV</nav><div class="sidebar-heading secondary-heading">RUST INDEX</div><nav aria-label="Rust concepts"><a href="#03-storing-requests/a-string-owns-its-bytes">String and moves</a><a href="#03-storing-requests/borrow-while-drawing">Shared borrowing</a><a href="#03-storing-requests/why-store-ids">Vec and typed IDs</a><a href="#07-values-and-functions">Values and functions</a><a href="#08-structs-vectors-and-ids">Structs and collections</a><a href="#09-options-results-and-matching">Option, Result, and match</a><a href="#24-sharing-work">Channels, locks, and atomics</a><a href="#23-shaping-the-program">Architecture and alternatives</a></nav><p class="sidebar-note">A living explanation of the program, its Rust concepts, and the evidence behind its decisions.</p><div class="sidebar-status"><span class="status-dot"></span> Build in progress<br><small>Offline inspection · host summaries</small></div><p id="search-empty" hidden>No matching chapters.</p></aside><main id="reading" tabindex="-1">ARTICLES<div class="page-navigation"><a id="previous-chapter"></a><a id="next-chapter"></a></div></main><aside class="outline"><div class="sidebar-heading">ON THIS PAGE</div>OUTLINES<div class="outline-note">Read → predict → run → explain</div></aside></div></body></html>'''
page = page.replace('NAV', '\n'.join(nav)).replace('ARTICLES', '\n'.join(articles)).replace('OUTLINES', '\n'.join(outlines))
(DOCS / 'index.html').write_text(page)
print(f'Built {len(chapters)} chapters → docs/index.html')
