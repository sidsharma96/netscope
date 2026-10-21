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
    ['cargo','run','-p','netscope','--example','check_campaign','--locked','--offline','--','tests/fixtures/campaign/native-120'],
    [sys.executable, '-m', 'unittest', 'discover', '-s', 'scripts', '-p', 'test_doc_tools.py'],
    [sys.executable, 'scripts/check_docs.py'],
]
for command in commands:
    print('Running: ' + ' '.join(command), flush=True)
    subprocess.run(command, cwd=root, check=True)
