/** Controlled H2 and HTTP/1.1 TLS exchanges; trust exceptions stay in test browsers. */
import {createSecureServer} from 'node:http2';
import {createHash} from 'node:crypto';
import {readFileSync,appendFileSync,writeFileSync} from 'node:fs';
const [root,key,cert]=process.argv.slice(2);
const cases=new Map(JSON.parse(readFileSync(root+'/cases.json')).map(c=>[c.id,c]));
const server=createSecureServer({key:readFileSync(key),cert:readFileSync(cert),allowHTTP1:true});
server.on('request',(request,response)=>{
 const id=new URL(request.url,'https://localhost').pathname.split('/').at(-1);
 const c=cases.get(id);
 if(!c){response.writeHead(200,{'content-type':'text/html'});response.end('<!doctype html><link rel="icon" href="data:,">Controlled TLS trust prime');return;}
 const pieces=[];request.on('data',d=>pieces.push(d));
 request.on('end',()=>{
  appendFileSync(root+'/server-tls.jsonl',JSON.stringify({id,method:request.method,path:request.url,headers:request.rawHeaders,body_base64:Buffer.concat(pieces).toString('base64'),status:request.method==='OPTIONS'?204:c.status,transport:request.httpVersion,alpn:request.socket.alpnProtocol})+'\n');
  if(request.method==='OPTIONS'){response.writeHead(204,{'access-control-allow-origin':'*','access-control-allow-methods':'GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS','access-control-allow-headers':'Content-Type,X-Campaign','content-length':0});response.end();return;}
  const raw=Buffer.from(c.wire_base64,'base64');
  const headers={'content-type':c.mime,'cache-control':'no-store','access-control-allow-origin':'*','content-length':c.status===204||c.status===304?0:raw.length};
  if(c.gzip)headers['content-encoding']='gzip';
  if(c.duplicate)headers['x-repeat']=['one','two'];
  response.writeHead(c.status,headers);
  if(request.method==='HEAD'||c.status===204||c.status===304){response.end();return;}
  response.end(raw);
 });
});
server.on('upgrade',(request,socket)=>{
 const id=new URL(request.url,'https://localhost').pathname.split('/').at(-1);
 const c=cases.get(id);
 if(!c||c.family!=='W'){socket.end('HTTP/1.1 404 Missing\r\nContent-Length: 0\r\n\r\n');return;}
 appendFileSync(root+'/server-tls.jsonl',JSON.stringify({id,method:request.method,path:request.url,headers:request.rawHeaders,body_base64:'',status:101,transport:request.httpVersion,alpn:socket.alpnProtocol})+'\n');
 const accept=createHash('sha1').update(request.headers['sec-websocket-key']+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
 let headers='HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: '+(c.name==='connection-token-case'?'keep-alive, UpGrAdE':'Upgrade')+'\r\nSec-WebSocket-Accept: '+accept+'\r\n';
 if(c.name==='subprotocol-chat')headers+='Sec-WebSocket-Protocol: chat\r\n';
 if(c.name==='graphql-transport-ws')headers+='Sec-WebSocket-Protocol: graphql-transport-ws\r\n';
 socket.write(headers+'\r\n');
 const data=c.name==='binary'?Buffer.from([0,255,1,254]):Buffer.from(c.name==='unicode'?'café 📚':c.name==='graphql-transport-ws'?'{"type":"connection_ack"}':'book');
 socket.write(Buffer.concat([Buffer.from([c.name==='binary'?0x82:0x81,data.length]),data]));
 socket.once('data',()=>socket.end(Buffer.from([0x88,2,3,0xe8])));
 socket.on('error',()=>{});
 socket.setTimeout(5000,()=>socket.destroy());
});
server.listen(0,'127.0.0.1',()=>writeFileSync(root+'/tls-url','https://127.0.0.1:'+server.address().port));
