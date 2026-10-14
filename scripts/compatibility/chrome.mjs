/** Export controlled traffic using a disposable Chrome profile and native HAR writer. */
import {spawn} from 'node:child_process';
import fs from 'node:fs/promises';
const root=process.argv[2];const profile=root+'/chrome-profile';await fs.mkdir(profile,{recursive:true});
await fs.rm(profile+'/DevToolsActivePort',{force:true});
const proc=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--remote-debugging-port=0','--user-data-dir='+profile,'--no-first-run','--no-default-browser-check','--remote-allow-origins=*','--disable-background-networking','--disable-component-update','--ignore-certificate-errors-spki-list='+process.argv[3],'about:blank'],{stdio:'ignore'});
const wait=ms=>new Promise(r=>setTimeout(r,ms));
const sockets=[];
async function client(url){const ws=new WebSocket(url);sockets.push(ws);await new Promise((r,j)=>{ws.onopen=r;ws.onerror=j});let id=0;const pending=new Map();ws.onmessage=e=>{const m=JSON.parse(e.data);if(pending.has(m.id)){const [r,j]=pending.get(m.id);pending.delete(m.id);m.error?j(m.error):r(m.result)}};return (method,params={})=>new Promise((r,j)=>{const i=++id;pending.set(i,[r,j]);ws.send(JSON.stringify({id:i,method,params}))});}
try {
 let port;for(let i=0;i<100;i++){try {port=(await fs.readFile(profile+'/DevToolsActivePort','utf8')).split('\n')[0];break}catch{await wait(100)}}
 const targets=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();const page=targets.find(t=>t.type==='page');const control=await client(page.webSocketDebuggerUrl);
 const {targetId}=await control('Target.createTarget',{url:`devtools://devtools/bundled/devtools_app.html?ws=127.0.0.1:${port}/devtools/page/${page.id}`});
 let dt;for(let i=0;i<50;i++){const ts=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();dt=ts.find(t=>t.id===targetId);if(dt)break;await wait(100)}
 const frontend=await client(dt.webSocketDebuggerUrl);
 async function evaluate(c,expression){const r=await c('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value}
 for(let i=0;i<50;i++){try{if(await evaluate(frontend,"(async()=>{const S=await import('./core/sdk/sdk.js');return !!S.TargetManager.TargetManager.instance().primaryPageTarget()})()"))break}catch{}await wait(100)}
 console.log('Frontend',await evaluate(frontend,"(async()=>{const U=await import('./ui/legacy/legacy.js');await U.InspectorView.InspectorView.instance().showPanel('network'); const L=await import('./models/logs/logs.js');L.NetworkLog.NetworkLog.instance(); const S=await import('./core/sdk/sdk.js');return S.TargetManager.TargetManager.instance().targets().map(t=>({id:t.id(),type:t.type()}));})()"));
 await control('Page.navigate',{url:(await fs.readFile(root+'/url','utf8')).trim()});
 for(let i=0;i<2400;i++){if(await evaluate(control,'document.title')==='Netscope capture complete')break;await wait(100)}
 if(await evaluate(control,'document.title')!=='Netscope capture complete')throw new Error('Campaign watchdog expired');
 await fs.writeFile(root+'/chrome-observations.json',JSON.stringify(await evaluate(control,'window.campaignResults'),null,2));
 await fs.writeFile(root+'/chrome-version.json',JSON.stringify(await control('Browser.getVersion'),null,2));
 await wait(1000);
 const text=await evaluate(frontend,`(async()=>{const H=await import('./models/har/har.js');const L=await import('./models/logs/logs.js');const C=await import('./core/common/common.js');return H.Writer.Writer.harStringForRequests(L.NetworkLog.NetworkLog.instance().requests(),{sanitize:false},new C.Progress.CompositeProgress(new C.Progress.Progress()));})()`);
 await fs.writeFile(root+'/chrome-native.har',text);console.log('Chrome native export',JSON.parse(text).log.entries.length);
}finally{for(const ws of sockets)ws.close();proc.kill();}
