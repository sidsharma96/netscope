'use strict';
const pages = [...document.querySelectorAll('[data-page]')];
const links = [...document.querySelectorAll('[data-chapter]')];
const menu = document.querySelector('#chapter-nav');
const menuToggle = document.querySelector('#menu-toggle');
function route() {
  const [requested, section] = location.hash.slice(1).split('/');
  const page = pages.find(p => p.dataset.page === requested) || pages[0];
  pages.forEach(p => { p.hidden = p !== page; });
  document.querySelectorAll('[data-outline]').forEach(p => { p.hidden = p.dataset.outline !== page.dataset.page; });
  links.forEach(a => {
    const selected = a.dataset.chapter === page.dataset.page;
    a.classList.toggle('active', selected);
    if (selected) a.setAttribute('aria-current', 'page'); else a.removeAttribute('aria-current');
  });
  const i = pages.indexOf(page);
  for (const [id, offset, label] of [['previous-chapter', -1, '←'], ['next-chapter', 1, '→']]) {
    const a = document.getElementById(id), adjacent = links[i + offset];
    a.hidden = !adjacent;
    if (adjacent) { a.href = adjacent.href; a.textContent = `${label} ${adjacent.textContent.trim()}`; }
  }
  document.title = `${page.querySelector('h1').textContent} — Netscope`;
  menu.classList.remove('open'); menuToggle.setAttribute('aria-expanded', 'false');
  if (section) {
    const target = [...page.querySelectorAll('[id]')].find(el => el.id === `${page.dataset.page}--${section}`);
    if (target) target.scrollIntoView();
  } else window.scrollTo(0, 0);
}
window.addEventListener('hashchange', route);
route();
menuToggle.addEventListener('click', () => {
  const open = menu.classList.toggle('open');
  menuToggle.setAttribute('aria-expanded', String(open));
});
const themeToggle = document.querySelector('#theme-toggle');
function theme(light) {
  document.body.classList.toggle('light', light);
  themeToggle.textContent = light ? 'Dark' : 'Light';
  themeToggle.setAttribute('aria-label', `Switch to ${light ? 'dark' : 'light'} theme`);
}
try { theme(localStorage.getItem('netscope-doc-theme') === 'light'); } catch { theme(false); }
themeToggle.addEventListener('click', () => {
  const light = !document.body.classList.contains('light'); theme(light);
  try { localStorage.setItem('netscope-doc-theme', light ? 'light' : 'dark'); } catch { /* Theme still works without storage. */ }
});
const search = document.querySelector('#search');
search.addEventListener('input', () => {
  const query = search.value.trim().toLowerCase();
  links.forEach((link, i) => { link.hidden = !pages[i].textContent.toLowerCase().includes(query); });
  document.querySelector('#search-empty').hidden = links.some(a => !a.hidden);
});
document.addEventListener('keydown', event => {
  if (event.key === '/' && !['INPUT', 'TEXTAREA'].includes(document.activeElement.tagName)) {
    event.preventDefault(); search.focus();
  }
  if (event.key === 'Escape') { search.blur(); menu.classList.remove('open'); menuToggle.setAttribute('aria-expanded', 'false'); }
});
const steps = [
  ['url', 'The variable url owns the String. It tracks the location of one allocated text buffer.', false],
  ['request_url', 'Assignment moves ownership to request_url. The old name url can no longer be used. The text is not cloned.', false],
  ['request_url', 'A drawing function borrows a shared reference. It reads the existing bytes while request_url remains their owner.', true],
  ['request_url', 'The drawing function returns. Its borrow is no longer needed. The owner can still use the string.', false]
];
let step = 0;
function showStep() {
  document.querySelector('#owner-name').textContent = steps[step][0];
  document.querySelector('#step-description').textContent = steps[step][1];
  document.querySelector('#borrow-box').hidden = !steps[step][2];
  document.querySelector('#step-count').textContent = `Step ${step + 1} / ${steps.length}`;
  document.querySelector('#step-prev').disabled = step === 0;
  document.querySelector('#step-next').disabled = step === steps.length - 1;
}
document.querySelector('#step-prev').addEventListener('click', () => { if (step > 0) { step--; showStep(); } });
document.querySelector('#step-next').addEventListener('click', () => { if (step < steps.length - 1) { step++; showStep(); } });
showStep();

document.querySelector('.skip-link').addEventListener('click', event => {
  event.preventDefault(); document.querySelector('#reading').focus(); document.querySelector('#reading').scrollIntoView();
});
