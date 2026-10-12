"""Real loopback HTTP and PTY checks; wall-clock watchdogs are not timing assertions."""
from pathlib import Path
import fcntl
import json
import os
import pty
import re
import select
import socket
import struct
import subprocess
import tempfile
import termios
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
subprocess.run(['cargo', 'build', '-p', 'netscope-tui', '--locked', '--offline'], cwd=ROOT, check=True)


def screen_text(output):
    """Reconstruct the cursor-positioned cells Ratatui writes, ignoring styles."""
    cells = [[' '] * 100 for _ in range(30)]
    row = column = 0
    tokens = re.split(r'(\x1b\[[0-9;?]*[A-Za-z])', output.decode(errors='replace'))
    for token in tokens:
        if token.startswith('\x1b['):
            command = token[-1]
            args = token[2:-1]
            if args.startswith('?'):
                continue
            values = [int(part or '0') for part in args.split(';')]
            first = values[0] or 1
            if command in ('H', 'f'):
                row = first - 1
                column = (values[1] if len(values) > 1 and values[1] else 1) - 1
            elif command == 'G': column = first - 1
            elif command == 'd': row = first - 1
            elif command == 'A': row -= first
            elif command == 'B': row += first
            elif command == 'C': column += first
            elif command == 'D': column -= first
            elif command == 'J' and values[0] == 2: cells = [[' '] * 100 for _ in range(30)]
            elif command == 'K' and 0 <= row < 30:
                for x in range(max(0, column), 100): cells[row][x] = ' '
        else:
            for char in token:
                if char == '\r': column = 0
                elif char == '\n': row += 1
                elif char >= ' ':
                    if 0 <= row < 30 and 0 <= column < 100: cells[row][column] = char
                    column += 1
    return '\n'.join(''.join(line) for line in cells)


def check(mode):
    listener = socket.socket()
    listener.bind(('127.0.0.1', 0))
    listener.listen()
    listener.settimeout(15)
    received = threading.Event()
    closed = threading.Event()
    errors = []
    requests = []
    post_body = ' {"message":"café","n":1.00}\n'.encode()

    def serve():
        try:
            with listener.accept()[0] as stream:
                stream.settimeout(15)
                head = b''
                while not head.endswith(b'\r\n\r\n'):
                    part = stream.recv(1)
                    if not part:
                        raise AssertionError('closed before request')
                    head += part
                requests.append(head)
                if mode in ('post', 'saved'):
                    fields = dict(line.split(b':', 1) for line in head.split(b'\r\n')[1:] if b':' in line)
                    assert fields[b'content-type'].strip() == b'application/json'
                    assert int(fields[b'content-length']) == len(post_body)
                    body = bytearray()
                    while len(body) < len(post_body):
                        part = stream.recv(len(post_body) - len(body))
                        assert part, 'body ended early'
                        body.extend(part)
                    assert bytes(body) == post_body, body
                if mode in ('complete', 'post', 'saved'):
                    stream.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\nlocal response')
                else:
                    stream.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\npartial')
                received.set()
                try:
                    assert stream.recv(1) == b'', 'unexpected second request'
                except ConnectionResetError:
                    pass
                closed.set()
        except Exception as error:
            errors.append(error)
        finally:
            listener.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    master, slave = pty.openpty()
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix='netscope-test-') as temp:
            capture = json.loads((ROOT / 'tests/fixtures/har/single-request.har').read_text())
            capture['log']['entries'][0]['request']['url'] = f'http://127.0.0.1:{listener.getsockname()[1]}/local?q=1'
            path = Path(temp) / 'local.har'
            path.write_text(json.dumps(capture))
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
            before = termios.tcgetattr(slave)
            args = [str(path)]
            if mode == 'saved':
                saved_path = Path(temp) / 'saved.json'
                saved_path.write_text(json.dumps(dict(version=1, name='Local saved request',
                    url=capture['log']['entries'][0]['request']['url'], method='POST', format='json',
                    body=post_body.decode(), headers=[])))
                args = ['--draft', str(saved_path)]
            process = subprocess.Popen([str(ROOT / 'target/debug/netscope'), *args], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            output = bytearray()
            stage = 0
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if select.select([master], [], [], .05)[0]:
                    output.extend(os.read(master, 65536))
                visible = screen_text(output)
                if stage == 0 and 'NETSCOPE' in visible:
                    if mode == 'saved':
                        assert not requests, 'opening sent without review'
                        os.write(master, b'jj\rs')
                    elif mode == 'post':
                        os.write(master, b'\rrmtb' + post_body.replace(b'\n', b'\x0e') + b'\rjjjj\rs')
                    else:
                        os.write(master, b'\rrjjjj\rs')
                    stage = 1
                if stage == 1 and received.is_set():
                    if mode in ('complete', 'post', 'saved'):
                        if 'Outcome: Complete' not in visible or 'local response' not in visible:
                            continue
                        if mode == 'saved':
                            assert 'Saved draft' in visible and 'no capture' in visible
                            os.write(master, b'q')
                        else:
                            os.write(master, b'c')
                    elif mode == 'cancel':
                        os.write(master, b'sssss\x1b')
                    else:
                        os.write(master, b'q' if mode == 'quit' else b'\x03')
                    stage = 2
                if mode in ('complete', 'post') and stage == 2 and 'RESPONSE COMPARISON' in visible and 'Stored bytes differ' in visible:
                    assert 'Captured response' in visible and 'Replay response' in visible
                    assert 'Stored bytes differ' in visible
                    os.write(master, b'\x1b')
                    stage = 3
                if mode in ('complete', 'post') and stage == 3 and 'NEW RESULT' in visible:
                    os.write(master, b'q')
                    stage = 4
                if mode == 'cancel' and stage == 2 and 'Operation stopped' in visible:
                    os.write(master, b'o')
                    stage = 3
                if mode == 'cancel' and stage == 3 and 'Outcome: Cancelled' in visible:
                    os.write(master, b'q')
                    stage = 4
                if process.poll() is not None:
                    while select.select([master], [], [], 0)[0]:
                        output.extend(os.read(master, 65536))
                    break
            if process.poll() != 0:
                Path('/tmp/netscope-replay-pty.log').write_bytes(output)
            assert process.poll() == 0, f'{mode}: process did not exit cleanly (stage {stage})'
            assert stage >= 2, 'live send was not reached'
            if mode in ('complete', 'post'): assert stage == 4, 'comparison return was not exercised'
            thread.join(2)
            assert not thread.is_alive(), 'server did not observe socket disposal'
            assert not errors, errors
            assert closed.is_set(), 'socket was not closed'
            assert len(requests) == 1
            method = b'POST' if mode in ('post', 'saved') else b'GET'
            assert requests[0].startswith(method + b' /local?q=1 HTTP/1.1\r\n')
            assert b'accept-encoding: identity\r\n' in requests[0]
            after = termios.tcgetattr(slave)
            pending = getattr(termios, 'PENDIN', 0)
            before[3] &= ~pending
            after[3] &= ~pending
            assert before == after, 'terminal modes not restored'
            assert b'\x1b[?1049l' in output and b'\x1b[?25h' in output
            print(f'PASS: live {mode}; one {method.decode()}, connection disposed, terminal restored')
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
        listener.close()


for mode in ['complete', 'post', 'saved', 'cancel', 'quit', 'ctrl-c']:
    check(mode)


def check_save_and_reopen():
    with tempfile.TemporaryDirectory(prefix='netscope-save-') as temp:
        saved_path = Path(temp) / 'request.json'
        def drive(args, reopening=False):
            master, slave = pty.openpty()
            process = None
            try:
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
                before = termios.tcgetattr(slave)
                process = subprocess.Popen([str(ROOT / 'target/debug/netscope'), *args],
                    stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
                output = bytearray()
                stage = 0
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    if select.select([master], [], [], .05)[0]:
                        output.extend(os.read(master, 65536))
                    visible = screen_text(output)
                    if stage == 0 and 'NETSCOPE' in visible:
                        if reopening:
                            if 'Saved draft' not in visible or 'no capture' not in visible:
                                continue
                            os.write(master, b'q')
                            stage = 4
                        else:
                            os.write(master, b'\rrwquiet book\r' + str(saved_path).encode() + b'\r')
                            stage = 1
                    elif stage == 1 and 'Exact file contents follow' in visible:
                        assert not saved_path.exists(), 'file written before confirmation'
                        os.write(master, b'\x1b')
                        stage = 2
                    elif stage == 2 and 'Save cancelled' in visible:
                        assert not saved_path.exists(), 'cancel wrote a file'
                        os.write(master, b'wquiet book\r' + str(saved_path).encode() + b'\r')
                        stage = 3
                    elif stage == 3 and 'Exact file contents follow' in visible:
                        os.write(master, b'\r')
                        stage = 4
                    elif stage == 4 and not reopening and 'Draft saved' in visible:
                        data = json.loads(saved_path.read_text())
                        assert data['name'] == 'quiet book' and data['version'] == 1
                        os.write(master, b'q')
                        stage = 5
                    if process.poll() is not None:
                        while select.select([master], [], [], 0)[0]:
                            output.extend(os.read(master, 65536))
                        break
                assert process.poll() == 0, f'save/reopen did not exit at stage {stage}'
                assert stage == (4 if reopening else 5)
                after = termios.tcgetattr(slave)
                pending = getattr(termios, 'PENDIN', 0)
                before[3] &= ~pending
                after[3] &= ~pending
                assert before == after, 'save/reopen terminal attributes not restored'
                assert b'\x1b[?1049l' in output and b'\x1b[?25h' in output
            finally:
                if process is not None and process.poll() is None:
                    process.kill()
                    process.wait()
                os.close(master)
                os.close(slave)
        drive([str(ROOT / 'tests/fixtures/har/single-request.har')])
        drive(['--draft', str(saved_path)], reopening=True)
        print('PASS: save cancellation, confirmed file, independent reopen, terminal restored')


check_save_and_reopen()
