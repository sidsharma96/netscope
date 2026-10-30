"""Run lesson examples with the Cargo setup supplied to readers.

Uses cached dependencies and a temporary package. Examples marked no_run are
built but need the terminal check or a person to exercise their behavior.
"""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FENCE = re.compile(r'^```(rust[^\n]*|compile_fail[^\n]*)\n(.*?)^```', re.M | re.S)


def check():
    setup = (ROOT / 'docs/EXAMPLES.md').read_text()
    manifest = re.search(r'^```toml\n(.*?)^```', setup, re.M | re.S).group(1)
    manifest = manifest.replace('../netscope/crates/netscope-core',
                                (ROOT / 'crates/netscope-core').as_posix())
    env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / 'target/book-examples'))
    examples = []
    expected_failures = []
    pages = [ROOT / 'docs/EXAMPLES.md', *sorted((ROOT / 'docs/chapters').glob('*.md'))]
    with tempfile.TemporaryDirectory(prefix='netscope-book-') as temporary:
        folder = Path(temporary)
        (folder / 'src/bin').mkdir(parents=True)
        (folder / 'Cargo.toml').write_text(manifest)
        # Reuse the repository's resolved versions where the reader manifest permits.
        shutil.copyfile(ROOT / 'Cargo.lock', folder / 'Cargo.lock')
        for page in pages:
            text = page.read_text()
            for block in FENCE.finditer(text):
                kind, code = block.groups()
                line = text[:block.start()].count('\n') + 1
                label = f'{page.relative_to(ROOT)}:{line}'
                if not re.search(r'\bfn\s+main\s*\(', code):
                    code = 'fn main() {\n' + code + '\n}\n'
                if 'compile_fail' in {flag.strip() for flag in kind.split(',')}:
                    errors = re.findall(r'\bE\d{4}\b', kind)
                    if len(errors) != 1:
                        raise ValueError(f'Compile failure needs one expected diagnostic: {label}')
                    expected_failures.append((code, label, errors[0]))
                    continue
                name = f'lesson_{len(examples):02d}'
                (folder / f'src/bin/{name}.rs').write_text(code)
                examples.append((name, label, 'no_run' in kind))
        cargo = ['cargo', 'build', '--manifest-path', str(folder / 'Cargo.toml'),
                 '--bins', '--offline']
        subprocess.run(cargo, cwd=ROOT, env=env, check=True)
        for name, label, no_run in examples:
            if no_run:
                print(f'BUILT (interactive): {label}', flush=True)
            else:
                subprocess.run([str(Path(env['CARGO_TARGET_DIR']) / 'debug' / name)],
                               cwd=folder, check=True, timeout=15)
                print(f'RAN: {label}', flush=True)
        for code, label, error_code in expected_failures:
            source = folder / 'expected_failure.rs'
            source.write_text(code)
            result = subprocess.run(['rustc', '--edition=2024', str(source),
                                     '-o', str(folder / 'unexpected_success')],
                                    cwd=ROOT, capture_output=True, text=True)
            if result.returncode == 0 or f'error[{error_code}]' not in result.stderr:
                raise RuntimeError(f'Expected {error_code} did not occur: {label}\n{result.stderr}')
            print(f'EXPECTED {error_code}: {label}', flush=True)
    print(f'PASS: {len(examples)} examples built; '
          f'{sum(not no_run for _, _, no_run in examples)} ran; '
          f'{len(expected_failures)} intentional compile failure(s)')


if __name__ == '__main__':
    check()
