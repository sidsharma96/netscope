"""Separate POSIX pseudo-terminal check: q and an injected rendering error."""
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

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('NETSCOPE_CHECK_BINARY', ROOT / 'target/debug/netscope'))
WORKING_DIRECTORY = Path(os.environ.get('NETSCOPE_CHECK_CWD', ROOT))
subprocess.run(['cargo', 'build', '-p', 'netscope-tui', '--examples', '--bins', '--locked', '--offline'], cwd=ROOT, check=True)


def check(command, expected_exit, quit_key, small=False):
    master, slave = pty.openpty()
    process = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 10 if small else 30, 40 if small else 100, 0, 0))
        before = termios.tcgetattr(slave)
        process = subprocess.Popen(command, cwd=WORKING_DIRECTORY, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        output = bytearray()
        sent = False
        enlarged = False
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([master], [], [], .1)[0]:
                output.extend(os.read(master, 65536))
            if small and not enlarged and b'Terminal too small' in output:
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
                os.kill(process.pid, signal.SIGWINCH)
                enlarged = True
            if quit_key and not sent and b'NETSCOPE' in output:
                os.write(master, b'\x1b[6~\x1b[5~\x1b[F\x1b[Hh\rj\r\x1b[6~\x1b[F\x1b[H\x1b\x1ba/cover\r/\x15absent\r/\x15\r?q')
                sent = True
            if process.poll() is not None:
                while select.select([master], [], [], 0)[0]:
                    output.extend(os.read(master, 65536))
                break
        if process.poll() is None:
            raise AssertionError('terminal process timed out')
        if small:
            assert enlarged, 'small terminal did not recover'
        assert process.returncode == expected_exit, process.returncode
        assert b'NETSCOPE' in output, 'renderer was not reached'
        after = termios.tcgetattr(slave)
        # macOS may set PENDIN while restoring canonical input; it is a pending
        # input-retype flag, not a changed terminal mode. Compare all other bits.
        pending = getattr(termios, 'PENDIN', 0)
        before[3] &= ~pending
        after[3] &= ~pending
        assert after == before, 'terminal modes were not restored'
        assert b'\x1b[?1049h' in output, 'alternate screen was not entered'
        assert b'\x1b[?1049l' in output, 'alternate screen was not left'
        assert b'\x1b[?25h' in output, 'cursor was not shown on exit'
        if quit_key:
            assert sent, 'quit path was not exercised'
            # The title is emitted as cell differences, so HOSTS need not be
            # contiguous. This new border label replaces only blank rule cells.
            assert b'alphabetical' in output, 'host navigation was not exercised'
            assert b'URL search:' in output, 'search editor was not exercised'
        print('PASS:', 'keyboard exit' if quit_key else 'injected rendering error', 'restored terminal attributes, screen, and cursor')
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


capture = str(ROOT / 'tests/fixtures/har/multi-host.har')
check([str(BINARY), capture], 0, True)
check([str(BINARY), capture], 0, True, small=True)
check([str(ROOT / 'target/debug/examples/terminal_probe')], 1, False)
