import socket,json,subprocess,pathlib,time
import sys
R=pathlib.Path(sys.argv[1]);p=R/'ff-marionette';p.mkdir(exist_ok=True)
(p/'user.js').write_text('user_pref("marionette.port", 28289);\nuser_pref("browser.shell.checkDefaultBrowser", false);\nuser_pref("browser.startup.homepage_override.mstone", "ignore");\nuser_pref("datareporting.policy.dataSubmissionEnabled", false);\n')
proc=subprocess.Popen(['/Applications/Firefox.app/Contents/MacOS/firefox','--no-remote','--remote-allow-system-access','--marionette','--profile',str(p),'about:blank'],stdout=open(R/'ff-marionette.log','w'),stderr=subprocess.STDOUT)
s=None
try:
 for _ in range(100):
  try:s=socket.create_connection(('127.0.0.1',28289),timeout=1);break
  except OSError:time.sleep(.1)
 s.settimeout(40)
 def receive():
  length=b''
  while not length.endswith(b':'):length+=s.recv(1)
  n=int(length[:-1]);b=b''
  while len(b)<n:b+=s.recv(n-len(b))
  return json.loads(b)
 print(receive(),flush=True);counter=0
 def call(name,args={}):
  global counter
  counter+=1;b=json.dumps([0,counter,name,args]).encode();s.sendall(str(len(b)).encode()+b':'+b);r=receive()
  if r[2]:raise Exception(r[2])
  return r[3]
 session=call('WebDriver:NewSession',{'acceptInsecureCerts':True})
 (R/'firefox-version.json').write_text(json.dumps(session,indent=2))
 if session['capabilities']['acceptInsecureCerts'] is not True:raise RuntimeError('Test TLS capability was not applied')
 call('WebDriver:Navigate',{'url':(R/'tls-url').read_text()+'/prime'})
 call('Marionette:SetContext',{'value':'chrome'})
 loader="const {require}=ChromeUtils.importESModule('resource://devtools/shared/loader/Loader.sys.mjs');const {gDevTools}=require('devtools/client/framework/devtools');const win=Services.wm.getMostRecentWindow('navigator:browser');"
 def asyncjs(code):
  script="const done=arguments[arguments.length-1];(async()=>{"+loader+code+"})().then(done,e=>done({error:String(e),stack:e.stack}));"
  return call('WebDriver:ExecuteAsyncScript',{'script':script,'args':[]})
 print('toolbox',asyncjs("const t=await gDevTools.showToolboxForTab(win.gBrowser.selectedTab,{toolId:'netmonitor'});const panel=t.getPanel('netmonitor');const Actions=require('devtools/client/netmonitor/src/actions/index');panel.panelWin.store.dispatch(Actions.batchEnable(false));return 'ready';"),flush=True)
 call('Marionette:SetContext',{'value':'content'});call('WebDriver:Navigate',{'url':(R/'url').read_text()})
 for _ in range(2400):
  if call('WebDriver:GetTitle').get('value')=='Netscope capture complete':break
  time.sleep(.1)
 title=call('WebDriver:GetTitle')
 if title.get('value')!='Netscope capture complete':raise RuntimeError('Campaign watchdog expired')
 (R/'firefox-observations.json').write_text(json.dumps(call('WebDriver:ExecuteScript',{'script':'return window.campaignResults','args':[]}).get('value'),ensure_ascii=False,indent=2))
 call('Marionette:SetContext',{'value':'chrome'})
 print('settled',asyncjs("const t=gDevTools.getToolboxForTab(win.gBrowser.selectedTab);const p=t.getPanel('netmonitor'); for(let i=0;i<100;i++){if(p.panelWin.store.getState().requests.requests.size>=121)break;await new Promise(r=>win.setTimeout(r,100));} return Array.from(p.panelWin.store.getState().requests.requests.values()).map(r=>({url:r.url,status:r.status}));"),flush=True)
 print('inspect',asyncjs("const t=gDevTools.getToolboxForTab(win.gBrowser.selectedTab);const panel=t.getPanel('netmonitor');return {keys:Object.keys(panel),win:Object.keys(panel.panelWin).filter(k=>k.includes('store')||k.includes('connector'))};"),flush=True)
 result=asyncjs("const t=gDevTools.getToolboxForTab(win.gBrowser.selectedTab);const panel=t.getPanel('netmonitor');const {HarExporter}=require('devtools/client/netmonitor/src/har/har-exporter');const api=await t.getNetMonitorAPI();const connector=await api.getHarExportConnector();return await HarExporter.fetchHarData({connector,items:panel.panelWin.store.getState().requests.requests.values().toArray(),includeResponseBodies:true,forceExport:true});")
 value=result.get('value',result) if isinstance(result,dict) else result
 if isinstance(value,str):value=json.loads(value)
 (R/'firefox-native.har').write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
 print('Firefox entries',len(value['log']['entries']),flush=True)
finally:
 if s:s.close()
 proc.terminate();proc.wait(timeout=15)
