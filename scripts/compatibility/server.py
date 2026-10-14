"""Actual controlled browser exchanges, with independently retained request logs."""
import base64
import hashlib
import json
import struct
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit
from cases import cases

CASES=cases()
BY_ID={c['id']:c for c in CASES}
LOCK=threading.Lock()
ROOT=Path(__import__('sys').argv[1])
ROOT.mkdir(parents=True,exist_ok=True)
TLS_URL=__import__('sys').argv[2] if len(__import__('sys').argv)>2 else None

SCRIPT=r'''
const results=[];
async function run(c) {
 const url=(c.tls?TLS_URL:location.origin)+'/case/'+c.id+(c.query?'?'+c.query:'');
 const r={id:c.id,family:c.family};
 try {
  if(c.family==='W') {
   await new Promise(resolve=>{
    const protocols=c.name==='subprotocol-chat'?['chat']:c.name==='graphql-transport-ws'?['graphql-transport-ws']:undefined;
    const w=new WebSocket(url.replace(/^http/,'ws'),protocols);
    w.binaryType='arraybuffer'; r.messages=[];
    const timer=setTimeout(()=>{r.watchdog=true;w.close();resolve()},8000);
    w.onopen=()=>{r.open=true;if(c.name==='graphql-transport-ws')w.send(JSON.stringify({type:'connection_init'}));};
    w.onmessage=e=>{r.messages.push(typeof e.data==='string'?e.data:Array.from(new Uint8Array(e.data)));if(r.messages.length===(c.name==='two-messages'?2:1))w.close();};
    w.onerror=()=>{r.error=true};
    w.onclose=e=>{r.close=e.code;clearTimeout(timer);resolve()};
   });
  } else if(c.family==='S') {
   await new Promise(resolve=>{
    const stream=new EventSource(url);r.events=[];
    const timer=setTimeout(()=>{r.watchdog=true;stream.close();resolve()},8000);
    function event(e){r.events.push({type:e.type,data:e.data,id:e.lastEventId})}
    for(const type of ['message','added','done','update'])stream.addEventListener(type,event);
    stream.onerror=()=>{r.ended=true;stream.close();clearTimeout(timer);resolve()};
   });
  } else {
   const options={method:c.method,redirect:c.follow?'follow':'manual',headers:{}};
   if(c.request!==undefined) {options.body=c.request;options.headers['Content-Type']=c.request_mime||'text/plain'}
   if(c.header)options.headers[c.header]='synthetic';
   const response=await fetch(url,options);
   const data=new Uint8Array(await response.arrayBuffer());
   r.status=response.status;r.type=response.type;r.bytes=data.length;
   r.sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',data))).map(b=>b.toString(16).padStart(2,'0')).join('');
  }
 }catch(e){r.error=String(e)}
 results.push(r);
}
setTimeout(async()=>{for(const c of CASES)await run(c);window.campaignResults=results;document.title='Netscope capture complete'},1000);
'''

class Handler(BaseHTTPRequestHandler):
    protocol_version='HTTP/1.1'
    def log_message(self,*_):pass
    def record(self,c,body):
        row=dict(id=c['id'],method=self.command,path=self.path,headers=list(self.headers.items()),body_base64=base64.b64encode(body).decode(),status=c['status'],transport='http/1.1')
        with LOCK:
            with (ROOT/'server-http.jsonl').open('a') as f:f.write(json.dumps(row,ensure_ascii=False)+'\n')
    def response(self,c):
        length=int(self.headers.get('Content-Length','0'))
        body=self.rfile.read(length) if 0<=length<=32768 else b''
        self.record(c,body)
        if c['family']=='W':return self.websocket(c)
        status=c['status'];raw=base64.b64decode(c['wire_base64'])
        self.send_response(status)
        self.send_header('Content-Type',c['mime']);self.send_header('Cache-Control','no-store')
        if c.get('gzip'):self.send_header('Content-Encoding','gzip')
        if c.get('cookie'):self.send_header('Set-Cookie','campaign=synthetic; Path=/; SameSite=Lax')
        if c.get('duplicate'):
            self.send_header('X-Repeat','one');self.send_header('X-Repeat','two')
        if status==429:self.send_header('Retry-After','1')
        if c.get('follow'):self.send_header('Location','/target/'+c['id'])
        empty=self.command=='HEAD' or status in (204,304)
        chunked=c.get('chunked') and not empty
        self.send_header('Transfer-Encoding','chunked') if chunked else self.send_header('Content-Length',str(0 if status in (204,304) else len(raw)))
        self.end_headers()
        if not empty:
            pieces=[raw[:1],raw[1:7],raw[7:]] if c['family']=='S' or chunked else [raw]
            for piece in pieces:
                if not piece:continue
                self.wfile.write((f'{len(piece):x}\r\n'.encode()+piece+b'\r\n') if chunked else piece);self.wfile.flush()
            if chunked:self.wfile.write(b'0\r\n\r\n');self.wfile.flush()
    def websocket(self,c):
        name=c['name']
        if c['status']!=101:
            self.send_response(c['status']);self.send_header('Content-Length','0');self.end_headers();return
        accept=base64.b64encode(hashlib.sha1((self.headers.get('Sec-WebSocket-Key','')+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
        self.send_response(101)
        if name!='missing-upgrade':self.send_header('Upgrade','websocket')
        self.send_header('Connection','keep-alive, UpGrAdE' if name=='connection-token-case' else 'Upgrade')
        if name!='missing-accept':self.send_header('Sec-WebSocket-Accept','wrong' if name=='bad-digest' else accept)
        if name in ('subprotocol-chat','graphql-transport-ws','unoffered-protocol'):self.send_header('Sec-WebSocket-Protocol','chat' if name=='subprotocol-chat' else 'graphql-transport-ws' if name=='graphql-transport-ws' else 'unoffered')
        self.end_headers();self.connection.settimeout(3)
        def frame(opcode,payload,fin=True):
            size=len(payload);header=bytes([(0x80 if fin else 0)|opcode])+(bytes([size]) if size<126 else b'\x7e'+struct.pack('!H',size))
            self.wfile.write(header+payload);self.wfile.flush()
        try:
            if name in ('missing-accept','bad-digest','missing-upgrade','unoffered-protocol'):
                self.connection.recv(1024);return
            if name.startswith('close-'):frame(8,struct.pack('!H',1001 if name=='close-going-away' else 1000));return
            text=('café 📚' if name=='unicode' else '' if name=='empty-text' else 'x'*4096 if name=='long-text' else '{"type":"connection_ack"}' if name=='graphql-transport-ws' else '{"book":1}' if name=='json-message' else 'book').encode()
            if name=='ping-text':frame(9,b'ping')
            if name=='fragmented':frame(1,b'first ',False);frame(0,b'second')
            else:frame(2,bytes([0,255,1,254])) if name=='binary' else frame(1,text)
            if name=='two-messages':frame(1,b'second')
            self.connection.recv(4096);frame(8,b'\x03\xe8')
        except OSError:pass
        finally:self.close_connection=True
    def do_GET(self):
        path=urlsplit(self.path).path
        if path=='/':
            config=json.dumps(CASES,ensure_ascii=True).replace('</','<\/')
            tls=json.dumps(TLS_URL or '')
            page=('<!doctype html><meta charset="utf-8"><link rel="icon" href="data:,"><title>Netscope campaign</title><script>const CASES='+config+';const TLS_URL='+tls+';'+SCRIPT+'</script>').encode()
            self.send_response(200);self.send_header('Content-Type','text/html; charset=utf-8');self.send_header('Content-Length',str(len(page)));self.end_headers();self.wfile.write(page);return
        if path.startswith('/target/'):
            c=dict(BY_ID[path.split('/')[-1]],status=200,follow=False);return self.response(c)
        c=BY_ID.get(path.split('/')[-1])
        if not c:self.send_error(404);return
        self.response(c)
    do_POST=do_GET
    do_PUT=do_GET
    do_PATCH=do_GET
    do_DELETE=do_GET
    do_OPTIONS=do_GET
    do_HEAD=do_GET

if __name__=='__main__':
    (ROOT/'cases.json').write_text(json.dumps(CASES,ensure_ascii=False,indent=2)+'\n')
    with ThreadingHTTPServer(('127.0.0.1',0),Handler) as server:
        (ROOT/'url').write_text(f'http://127.0.0.1:{server.server_port}/')
        print((ROOT/'url').read_text(),flush=True)
        try:server.serve_forever()
        except KeyboardInterrupt:pass
