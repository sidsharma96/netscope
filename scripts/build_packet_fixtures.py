"""Build deterministic illustrative PCAPs; these are not recordings of real traffic."""
from pathlib import Path
import hashlib
import ipaddress
import struct

ROOT = Path(__file__).resolve().parents[1] / 'tests/fixtures/pcap'


def checksum(data):
    if len(data) % 2:
        data += b'\0'
    total = sum(struct.unpack('!' + 'H' * (len(data) // 2), data))
    while total >> 16:
        total = (total & 65535) + (total >> 16)
    return (~total) & 65535


def packet(payload, sequence=0, flags=0x18, source_port=9000, dest_port=80,
           reverse=False, udp=False, ipv6=False, fragment=False, vlan=False):
    source = ipaddress.ip_address('2001:db8::1' if ipv6 else '192.0.2.1').packed
    dest = ipaddress.ip_address('2001:db8::2' if ipv6 else '198.51.100.2').packed
    if reverse:
        source, dest = dest, source
        source_port, dest_port = dest_port, source_port
    protocol = 17 if udp else 6
    if udp:
        header = struct.pack('!HHHH', source_port, dest_port, 8 + len(payload), 0)
    else:
        header = struct.pack('!HHIIBBHHH', source_port, dest_port, sequence, 0,
                             0x50, flags, 4096, 0, 0)
    data = header + payload
    pseudo = source + dest + (struct.pack('!I3xB', len(data), protocol) if ipv6
                             else struct.pack('!BBH', 0, protocol, len(data)))
    value = checksum(pseudo + data)
    position = 6 if udp else 16
    data = data[:position] + struct.pack('!H', value or 65535) + data[position + 2:]
    if ipv6:
        ip = struct.pack('!IHBB16s16s', 6 << 28, len(data), protocol, 64, source, dest)
    else:
        ip = struct.pack('!BBHHHBBH4s4s', 0x45, 0, 20 + len(data), 1,
                         0x2000 if fragment else 0, 64, protocol, 0, source, dest)
        ip = ip[:10] + struct.pack('!H', checksum(ip)) + ip[12:]
    ethernet = bytes.fromhex('020000000002020000000001')
    ethernet += bytes.fromhex('8100000786dd' if ipv6 else '810000070800') if vlan else struct.pack('!H', 0x86dd if ipv6 else 0x0800)
    return ethernet + ip + data


def pcap(records, little=True, nano=False, link=1):
    order = '<' if little else '>'
    magic = 0xa1b23c4d if nano else 0xa1b2c3d4
    out = bytearray(struct.pack(order + 'IHHIIII', magic, 2, 4, 0, 0, 262144, link))
    for index, item in enumerate(records):
        data, original = item if isinstance(item, tuple) else (item, len(item))
        out.extend(struct.pack(order + 'IIII', 1700000000 + index, 123456789 if nano else 123456,
                               len(data), original))
        out.extend(data)
    return bytes(out)


def build():
    ROOT.mkdir(parents=True, exist_ok=True)
    get = b'GET /books HTTP/1.1\r\nHost: example.test\r\n\r\n'
    sdp = b'v=0\r\no=- 1 1 IN IP4 192.0.2.1\r\ns=Illustration\r\nt=0 0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=ice-ufrag:sample\r\na=fingerprint:sha-256 00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00\r\n'
    cut = 47
    chunks = (f'{cut:x};piece=one\r\n'.encode() + sdp[:cut] + b'\r\n'
              + f'{len(sdp)-cut:x}\r\n'.encode() + sdp[cut:] + b'\r\n0\r\nX-End: yes\r\n\r\n')
    response = b'HTTP/1.1 200 OK\r\nContent-Type: application/sdp\r\nTransfer-Encoding: chunked\r\n\r\n' + chunks
    records = [packet(b'', 100, 2), packet(b'', 500, 0x12, reverse=True),
               packet(get[13:], 114), packet(get[:13], 101), packet(get[:13], 101),
               packet(b'', 101 + len(get), 0x11),
               packet(response[90:], 591, reverse=True), packet(response[:90], 501, reverse=True),
               packet(b'', 501 + len(response), 0x11, reverse=True)]
    records += [packet(b'', 200, 2, source_port=9001), packet(b'GET /gap HTTP/1.1\r\n', 201, source_port=9001),
                packet(b'\r\n', 240, source_port=9001),
                packet(b'', 1000, 2, source_port=9002), packet(b'AB', 1001, source_port=9002), packet(b'AX', 1001, source_port=9002),
                packet(b'', 700, 2, source_port=9003)]
    full = packet(b'abcdef', 701, source_port=9003)
    records += [(full[:-3], len(full)), packet(b'', 0xfffffffe, 2, source_port=9004),
                packet(b'abc', 0xffffffff, source_port=9004), packet(b'', 2, 0x11, source_port=9004)]
    dnsname = b'\x05books\x04test\0'
    dns = struct.pack('!HHHHHH', 7, 0x8180, 1, 3, 0, 0) + dnsname + struct.pack('!HH', 1, 1)
    for address in ['203.0.113.10', '203.0.113.11', '2001:db8::8']:
        value = ipaddress.ip_address(address).packed
        dns += b'\xc0\x0c' + struct.pack('!HHIH', 1 if len(value) == 4 else 28, 1, 60, len(value)) + value
    records += [packet(dns, source_port=53000, dest_port=53, reverse=True, udp=True),
                packet(bytes.fromhex('000100002112a442') + b'example-id12', source_port=50000, dest_port=3478, udp=True, ipv6=True, vlan=True),
                packet(b'fragment', fragment=True), bytes.fromhex('0200000000020200000000010806') + b'illustrative ARP bytes']
    (ROOT / 'streams.pcap').write_bytes(pcap(records))
    (ROOT / 'big-nano.pcap').write_bytes(pcap([packet(b'hello', ipv6=True)], little=False, nano=True))
    (ROOT / 'raw-ip.pcap').write_bytes(pcap([packet(b'raw')[14:]], link=101))
    (ROOT / 'unknown-link.pcap').write_bytes(pcap([b'opaque'], link=147))
    # Loopback (DLT_NULL) stores the address family in the capturing host's byte
    # order: 2 is AF_INET everywhere, 30 is AF_INET6 on macOS, 10 on Linux.
    for little, name in [(True, 'loopback-null.pcap'), (False, 'loopback-null-be.pcap')]:
        family = '<I' if little else '>I'
        (ROOT / name).write_bytes(pcap([struct.pack(family, 2) + packet(b'loop4')[14:],
                                        struct.pack(family, 30) + packet(b'loop6', ipv6=True)[14:]],
                                       little=little, link=0))
    # Linux cooked captures (tcpdump -i any): SLL is 16 bytes ending in the
    # EtherType; SLL2 is 20 bytes starting with it. Both are big-endian fields.
    sll = struct.pack('!HHH8sH', 0, 1, 6, bytes.fromhex('0200000000010000'), 0x0800)
    sll2 = struct.pack('!HHIHBB8s', 0x86dd, 0, 2, 1, 0, 6, bytes.fromhex('0200000000010000'))
    (ROOT / 'linux-sll.pcap').write_bytes(pcap([sll + packet(b'cooked')[14:]], link=113))
    (ROOT / 'linux-sll2.pcap').write_bytes(pcap([sll2 + packet(b'cooked2', ipv6=True)[14:]], link=276))
    hashes = []
    for path in sorted(ROOT.glob('*.pcap')):
        hashes.append(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}')
    (ROOT / 'SHA256SUMS').write_text('\n'.join(hashes) + '\n')


if __name__ == '__main__':
    build()
