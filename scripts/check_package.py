"""Build registry archives and check the files needed outside this checkout."""
from pathlib import Path
import argparse
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--allow-dirty', action='store_true', help='Check local edits before committing')
args = parser.parse_args()
command = ['cargo', 'package', '--workspace', '--locked', '--offline']
if args.allow_dirty:
    command.append('--allow-dirty')
subprocess.run(command, cwd=ROOT, check=True)
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
for manifest in sorted((ROOT / 'crates').glob('*/Cargo.toml')):
    name = tomllib.loads(manifest.read_text())['package']['name']
    prefix = f'{name}-{version}'
    with tarfile.open(ROOT / 'target/package' / f'{prefix}.crate') as archive:
        files = {m.name.removeprefix(prefix + '/'): m for m in archive.getmembers()}
        def content(path):
            member = files[path]
            assert member.isfile(), f'{name}: {path} must be a regular archived file'
            stream = archive.extractfile(member)
            assert stream is not None
            return stream.read()
        for license_name in ['LICENSE-MIT', 'LICENSE-APACHE']:
            assert content(license_name) == (ROOT / license_name).read_bytes()
        packaged = tomllib.loads(content('Cargo.toml').decode())
        assert packaged['package']['license'] == 'MIT OR Apache-2.0'
        assert packaged['package']['version'] == version
        for dep in packaged.get('dependencies', {}).values():
            assert 'path' not in dep, f'{name}: packaged dependency still has a local path'
        assert not any(p.startswith(('tests/', 'examples/')) for p in files)
        assert not any(p.startswith('.') and p != '.cargo_vcs_info.json' for p in files)
    print(f'PASS: {prefix}; archive builds, licenses match, dependencies use registry versions')
