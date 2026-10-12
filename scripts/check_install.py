"""Install into a temporary root and run from outside the checkout."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
with tempfile.TemporaryDirectory(prefix='netscope-install-') as temporary:
    folder = Path(temporary)
    install = folder / 'installation'
    elsewhere = folder / 'working directory'
    elsewhere.mkdir()
    subprocess.run(['cargo', 'install', '--path', str(ROOT / 'crates/netscope-tui'),
                    '--root', str(install), '--locked', '--offline'], cwd=ROOT, check=True, timeout=600)
    binary = install / 'bin' / ('netscope.exe' if os.name == 'nt' else 'netscope')
    assert binary.is_file()
    environment = os.environ.copy()
    environment['PATH'] = str(binary.parent) + os.pathsep + environment.get('PATH', '')
    def run(*args, status=0):
        output = subprocess.run(['netscope', *args], cwd=elsewhere, env=environment,
                                capture_output=True, text=True, timeout=10)
        assert output.returncode == status, (args, output.returncode, output.stderr)
        assert '\x1b' not in output.stdout + output.stderr
        return output
    assert run('--version').stdout == f'netscope {VERSION}\n'
    assert 'Opening never sends' in run('--help').stdout
    assert 'Usage:' in run().stdout
    shutil.copyfile(ROOT / 'tests/fixtures/har/single-request.har', elsewhere / '-capture.har')
    shutil.copyfile(ROOT / 'tests/fixtures/drafts/book.json', elsewhere / '-request.json')
    assert 'Summary mode' in run('--', '-capture.har').stdout
    assert 'No request sent' in run('--draft', '--', '-request.json').stdout
    assert run('--unknown', status=2).stdout == ''
    run('missing.har', status=1)
    (elsewhere / 'bad.json').write_text('{')
    run('--draft', 'bad.json', status=1)
    print('PASS: release installation, PATH lookup, help/version, local files, and exit codes outside checkout')
