"""Generate fixed synthetic workloads and report local release-build measurements."""
import copy
import datetime
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def capture(count, scenario):
    template = json.loads((ROOT / 'tests/fixtures/har/single-request.har').read_text())
    seed = template['log']['entries'][0]
    entries = []
    start = datetime.datetime(2026, 10, 9, 12, tzinfo=datetime.timezone.utc)
    for i in range(count):
        entry = copy.deepcopy(seed)
        entry['startedDateTime'] = (start + datetime.timedelta(milliseconds=i)).isoformat()
        entry['request']['url'] = f'https://host{i % 100:03}.example.test/books/{i}'
        if scenario != 'spread':
            entry['request']['url'] = 'https://example.test/repeated'
            entry['time'] = 10000
            entry['response']['status'] = 503 if scenario == 'overlapping-retries' else 307
            if scenario == 'overlapping-redirects':
                entry['response']['headers'].append({'name': 'Location', 'value': '/repeated'})
        entries.append(entry)
    template['log']['entries'] = entries
    return json.dumps(template, separators=(',', ':'), ensure_ascii=False).encode()


if __name__ == '__main__':
    subprocess.run(['cargo', 'build', '--release', '-p', 'netscope', '--example', 'capture_probe', '--locked', '--offline'], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix='netscope-large-') as directory:
        for scenario in ('spread', 'overlapping-retries', 'overlapping-redirects'):
            for count in (1000, 10000):
                path = Path(directory) / f'{scenario}-{count}.har'
                path.write_bytes(capture(count, scenario))
                print(f'{scenario}:', flush=True)
                for _ in range(3):
                    subprocess.run([str(ROOT / 'target/release/examples/capture_probe'), str(path)], cwd=ROOT, check=True)
