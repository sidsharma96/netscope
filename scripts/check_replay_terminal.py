"""Real loopback HTTP and PTY checks; wall-clock watchdogs are not timing assertions."""
from pathlib import Path
import fcntl
import json
import os
import pty
import select
import socket
import struct
import subprocess
import tempfile
import termios
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('NETSCOPE_CHECK_BINARY', ROOT / 'target/debug/netscope'))
WORKING_DIRECTORY = Path(os.environ.get('NETSCOPE_CHECK_CWD', ROOT))
if 'NETSCOPE_CHECK_BINARY' not in os.environ:
    subprocess.run(['cargo', 'build', '-p', 'netscope', '--locked', '--offline'], cwd=ROOT, check=True)


from terminal_cells import screen_text


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
                if mode == 'stream':
                    stream.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1000\r\n\r\nid: live\ndata: first event\n\ndata: pending\n')
                elif mode in ('complete', 'post', 'saved'):
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
            process = subprocess.Popen([str(BINARY), *args], cwd=WORKING_DIRECTORY, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
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
                        os.write(master, b'jjj\rs')
                    elif mode == 'post':
                        # Correct the final letter in the middle of the body before sending.
                        typed = post_body.replace('café'.encode(), b'cafx').replace(b'\n', b'\x0e')
                        position = post_body.decode().index('café') + 3
                        correction = b'\x1b[H' + b'\x1b[C' * position + b'\x1b[3~' + 'é'.encode() + b'\x1b[F'
                        os.write(master, b'/local\r\rrmtb' + typed + correction + b'\rjjjj\rs')
                    elif mode == 'stream':
                        os.write(master, b'/local\r\rrjjjj\rl')
                    else:
                        os.write(master, b'/local\r\rrjjjj\rs')
                    stage = 1
                if mode == 'stream' and stage == 1 and 'REVIEW LIVE SSE' in visible and '30 seconds total' in visible:
                    assert not requests, 'reviewing live mode sent a request'
                    os.write(master, b's')
                    stage = 10
                if mode == 'stream' and stage == 10 and 'first event' in visible and 'Observed +' in visible:
                    assert received.is_set()
                    os.write(master, b'\x1b')
                    stage = 2
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
                if mode in ('cancel', 'stream') and stage == 2 and 'Operation stopped' in visible:
                    os.write(master, b'o')
                    stage = 3
                if mode in ('cancel', 'stream') and stage == 3 and 'Outcome: Cancelled' in visible:
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
            if mode in ('complete', 'post', 'stream'): assert stage == 4, 'comparison return was not exercised'
            thread.join(2)
            assert not thread.is_alive(), 'server did not observe socket disposal'
            assert not errors, errors
            assert closed.is_set(), 'socket was not closed'
            assert len(requests) == 1
            method = b'POST' if mode in ('post', 'saved') else b'GET'
            assert requests[0].startswith(method + b' /local?q=1 HTTP/1.1\r\n')
            assert b'accept-encoding: identity\r\n' in requests[0]
            if mode == 'stream': assert b'accept: text/event-stream\r\n' in requests[0]
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


for mode in ['complete', 'post', 'saved', 'cancel', 'quit', 'ctrl-c', 'stream']:
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
                process = subprocess.Popen([str(BINARY), *args],
                    cwd=WORKING_DIRECTORY, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
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


def check_captured_sse():
    master, slave = pty.openpty()
    process = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
        before = termios.tcgetattr(slave)
        process = subprocess.Popen([str(BINARY), str(ROOT / 'tests/fixtures/har/sse.har')],
            cwd=WORKING_DIRECTORY, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        output = bytearray()
        stage = 0
        seen = set()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([master], [], [], .05)[0]:
                output.extend(os.read(master, 65536))
            visible = screen_text(output)
            if stage == 0 and 'NETSCOPE' in visible:
                os.write(master, b'\r3')
                stage = 1
            if stage == 1 and 'Inspector · Body' in visible:
                # Inspect real wrapped cells while scrolling; the observations
                # need not all fit in the same viewport.
                detail = ''.join(line[41:99].rstrip() for line in visible.splitlines()[8:28])
                markers = ('SSE EVENTS', 'first line\\ncafé', 'Unfinished trailing block',
                           'instruction, not an observed retry')
                seen.update(marker for marker in markers if ''.join(marker.split()) in ''.join(detail.split()))
                if len(seen) == len(markers):
                    os.write(master, b'q')
                    stage = 3
                else:
                    os.write(master, b'j')
            if process.poll() is not None:
                break
        assert stage == 3, f'captured SSE inspection missed markers: {set(markers) - seen}'
        assert process.wait(timeout=3) == 0
        while select.select([master], [], [], 0)[0]:
            output.extend(os.read(master, 65536))
        after = termios.tcgetattr(slave)
        pending = getattr(termios, 'PENDIN', 0)
        before[3] &= ~pending
        after[3] &= ~pending
        assert after == before, 'SSE inspection did not restore terminal modes'
        assert b'\x1b[?1049l' in output and b'\x1b[?25h' in output
        print('PASS: retained SSE events, unfinished tail, retry label, and terminal restoration')
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


check_captured_sse()
