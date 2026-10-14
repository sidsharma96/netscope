"""Install into a temporary root and run from outside the checkout."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import sys
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
    (elsewhere / 'empty.har').write_text('{"log":{"version":"1.2","creator":{"name":"check","version":"1"},"entries":[]}}')
    assert '0 requests' in run('empty.har').stdout
    (elsewhere / 'bad.har').write_text('{')
    run('bad.har', status=1)
    shutil.copyfile(ROOT / 'tests/fixtures/har/single-request.har', elsewhere / 'invalid.har')
    (elsewhere / 'invalid.har').write_text((elsewhere / 'invalid.har').read_text().replace('45', '-45', 1))
    run('invalid.har', status=1)
    shutil.copyfile(ROOT / 'tests/fixtures/pcap/streams.pcap', elsewhere / '-packets.pcap')
    packet_text = run('--pcap', '--', '-packets.pcap').stdout
    assert '24 packets' in packet_text and 'Chunked terminator' in packet_text
    run('--pcap', 'missing.pcap', status=1)
    (elsewhere / 'bad.pcap').write_bytes(b'not a capture')
    run('--pcap', 'bad.pcap', status=1)
    environment['NETSCOPE_CHECK_BINARY'] = str(binary)
    environment['NETSCOPE_CHECK_CWD'] = str(elsewhere)
    if os.name == 'posix':
        for script in ('check_terminal.py', 'check_replay_terminal.py', 'check_packet_terminal.py'):
            subprocess.run([sys.executable, str(ROOT / 'scripts' / script)], cwd=elsewhere,
                           env=environment, check=True, timeout=180)
    print('PASS: release installation, PATH lookup, help/version, local files, and exit codes outside checkout')
