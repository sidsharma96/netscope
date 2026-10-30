"""Small deterministic screen reconstruction for POSIX PTY checks."""
import re


def screen_text(output, width=100, height=30):
    """Reconstruct the cursor-positioned cells Ratatui writes, ignoring styles."""
    cells = [[' '] * width for _ in range(height)]
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
            elif command == 'J' and values[0] == 2: cells = [[' '] * width for _ in range(height)]
            elif command == 'K' and 0 <= row < height:
                for x in range(max(0, column), width): cells[row][x] = ' '
        else:
            for char in token:
                if char == '\r': column = 0
                elif char == '\n': row += 1
                elif char >= ' ':
                    if 0 <= row < height and 0 <= column < width: cells[row][column] = char
                    column += 1
    return '\n'.join(''.join(line) for line in cells)
