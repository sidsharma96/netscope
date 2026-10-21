"""Offline PCAP/PTTY workflows; watchdogs detect hangs, not performance."""
from pathlib import Path
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import termios
import time
from terminal_cells import screen_text

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('NETSCOPE_CHECK_BINARY', ROOT / 'target/debug/netscope'))
CWD = Path(os.environ.get('NETSCOPE_CHECK_CWD', ROOT))
if 'NETSCOPE_CHECK_BINARY' not in os.environ:
    subprocess.run(['cargo', 'build', '-p', 'netscope', '--locked', '--offline'], cwd=ROOT, check=True)

master, slave = pty.openpty()
process = None
try:
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen([str(BINARY), '--pcap', str(ROOT / 'tests/fixtures/pcap/streams.pcap')],
                               cwd=CWD, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
    output = bytearray()
    stage = 0
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if select.select([master], [], [], .05)[0]:
            output.extend(os.read(master, 65536))
        visible = screen_text(output)
        if stage == 0 and 'NETSCOPE / PACKETS' in visible and '24 packets' in visible:
            os.write(master, b'\r')
            stage = 1
        if stage == 1 and 'PACKET LAYERS' in visible and 'Sequence 100' in visible:
            os.write(master, b'\x1b')
            stage = 11
        if stage == 11 and 'NETSCOPE / PACKETS' in visible:
            os.write(master, b's')
            stage = 12
        if stage == 12 and 'TCP STREAMS' in visible and 'directed epochs' in visible:
            os.write(master, b'j\r')
            stage = 20
        if stage == 20 and 'THE BEGINNING IS READABLE' in visible:
            os.write(master, b'b')
            stage = 21
        if stage == 21 and 'BYTES · hex' in visible:
            os.write(master, b'2')
            stage = 2
        if stage == 2 and 'STREAM EVIDENCE' in visible and 'Chunked terminator' in visible:
            os.write(master, b'\x1b[6~')
            stage = 3
        if stage == 3 and 'SDP-like text' in visible and 'Trailer X-End' in visible and 'not proof' in visible:
            assert 'not proof' in visible
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 10, 50, 0, 0))
            os.kill(process.pid, signal.SIGWINCH)
            stage = 4
        if stage == 4 and 'resize to at least' in visible:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
            os.kill(process.pid, signal.SIGWINCH)
            stage = 5
        if stage == 5 and 'STREAM EVIDENCE' in visible and 'SDP-like text' in visible:
            os.write(master, b'\x1b')
            stage = 6
        if stage == 6 and 'TCP STREAMS' in visible:
            os.write(master, b'j\r2')
            stage = 7
        if stage == 7 and 'missing' in visible and 'Start unknown' not in visible and 'Directed epoch #2' in visible and 'HTTP inspection stopped' in visible:
            assert 'HTTP inspection stopped' in visible
            os.write(master, b'q')
            stage = 8
        if process.poll() is not None:
            while select.select([master], [], [], 0)[0]:
                output.extend(os.read(master, 65536))
            break
    if process.poll() != 0 or stage != 8:
        Path('/tmp/netscope-packet-pty.log').write_bytes(output)
    assert process.poll() == 0 and stage == 8, f'packet workflow failed at stage {stage}'
    after = termios.tcgetattr(slave)
    pending = getattr(termios, 'PENDIN', 0)
    before[3] &= ~pending
    after[3] &= ~pending
    assert before == after, 'terminal modes not restored'
    assert b'\x1b[?1049l' in output and b'\x1b[?25h' in output
    print('PASS: packet layers, chunk/SDP evidence, gap refusal, resize recovery, terminal restoration')
finally:
    if process is not None and process.poll() is None:
        process.kill()
        process.wait()
    os.close(master)
    os.close(slave)
