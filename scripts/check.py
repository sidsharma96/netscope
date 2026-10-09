"""Run the same foundation checks locally and in CI, without a live service."""
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
commands = [
    ['cargo', 'fmt', '--all', '--', '--check'],
    ['cargo', 'check', '--workspace', '--all-targets', '--locked', '--offline'],
    ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings'],
    ['cargo', 'test', '--workspace', '--locked', '--offline'],
    ['cargo', 'build', '--workspace', '--locked', '--offline'],
    [sys.executable, 'scripts/check_docs.py'],
]
for command in commands:
    print('Running: ' + ' '.join(command), flush=True)
    subprocess.run(command, cwd=root, check=True)
