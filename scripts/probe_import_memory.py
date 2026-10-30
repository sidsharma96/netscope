"""Report whole-process HAR import peaks for fixed inputs on macOS/Linux."""
import argparse
import csv
import json
import os
from pathlib import Path
import sys
import tempfile

from probe_large_capture import capture

ROOT = Path(__file__).resolve().parents[1]


def measure(binary, path):
    pid = os.fork()
    if pid == 0:
        with open(os.devnull, 'rb') as source, open(os.devnull, 'wb') as sink:
            os.dup2(source.fileno(), 0)
            os.dup2(sink.fileno(), 1)
            os.execv(str(binary), [str(binary), str(path)])
    _, status, usage = os.wait4(pid, 0)
    if os.waitstatus_to_exitcode(status) != 0:
        raise RuntimeError(f'import failed: {path.name}')
    # macOS reports bytes; Linux reports KiB.
    return usage.ru_maxrss if sys.platform == 'darwin' else usage.ru_maxrss * 1024


def run(binary, label):
    output = csv.writer(sys.stdout)
    output.writerow(['label', 'workload', 'input_bytes', 'sample', 'peak_bytes'])
    with tempfile.TemporaryDirectory(prefix='netscope-import-') as temporary:
        workloads = [('small-1000', capture(1000, 'spread')),
                     ('small-10000', capture(10000, 'spread'))]
        data = json.loads(workloads[-1][1])
        for entry in data['log']['entries']:
            entry['response']['content']['text'] = 'x' * 2600
        workloads.append(('bodies-10000', json.dumps(data, separators=(',', ':')).encode()))
        for name, contents in workloads:
            path = Path(temporary) / f'{name}.har'
            path.write_bytes(contents)
            for sample in range(1, 4):
                output.writerow([label, name, len(contents), sample, measure(binary, path)])
                sys.stdout.flush()


if __name__ == '__main__':
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('label')
    args.add_argument('--binary', type=Path, default=ROOT / 'target/release/netscope')
    options = args.parse_args()
    run(options.binary.resolve(), options.label)
