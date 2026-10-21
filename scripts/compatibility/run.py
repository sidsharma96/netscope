"""Record 120 real browser scenarios twice using native HAR exporters on loopback."""
import argparse
import base64
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from cases import cases

parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
root=args.output.resolve()
root.mkdir(parents=True,exist_ok=True)
if any(root.iterdir()):raise SystemExit('Output directory must be empty; preserve earlier campaigns')
script=Path(__file__).resolve().parent
(root/'cases.json').write_text(json.dumps(cases(),ensure_ascii=False,indent=2)+'\n')
subprocess.run(['cargo','build','-p','netscope','--example','check_campaign','--offline'],cwd=script.parents[1],check=True)
processes=[]
def wait_file(path):
    for _ in range(100):
        if path.exists():return path.read_text().strip()
        time.sleep(.1)
    raise RuntimeError(f'No startup file: {path.name}')
try:
    with tempfile.TemporaryDirectory(prefix='netscope-campaign-tls-') as temp:
        tls=Path(temp);key=tls/'key.pem';cert=tls/'cert.pem'
        subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-sha256','-days','2','-subj','/CN=localhost','-addext','subjectAltName=DNS:localhost,IP:127.0.0.1','-keyout',str(key),'-out',str(cert)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        pub=subprocess.check_output(['openssl','x509','-in',str(cert),'-pubkey','-noout'])
        der=subprocess.check_output(['openssl','pkey','-pubin','-outform','der'],input=pub)
        fingerprint=base64.b64encode(hashlib.sha256(der).digest()).decode()
        processes.append(subprocess.Popen(['node',str(script/'h2_server.mjs'),str(root),str(key),str(cert)],stdout=(root/'tls-server.log').open('w'),stderr=subprocess.STDOUT))
        url=wait_file(root/'tls-url')
        processes.append(subprocess.Popen([sys.executable,str(script/'server.py'),str(root),url],stdout=(root/'http-server.log').open('w'),stderr=subprocess.STDOUT))
        wait_file(root/'url')
        started=datetime.datetime.now(datetime.timezone.utc).isoformat()
        # Profiles and keys are disposable and never become committed fixtures.
        for browser,command in [('chrome',['node',str(script/'chrome.mjs'),str(root),fingerprint]),('firefox',[sys.executable,str(script/'firefox.py'),str(root)])]:
            with (root/f'{browser}-capture.log').open('w') as log:
                subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=300)
            print(f'PASS: {browser} native campaign export',flush=True)
        (root/'provenance.json').write_text(json.dumps(dict(started_utc=started,ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),scenarios=120,browser_runs=240,transport='loopback HTTP/1.1 and controlled self-signed HTTPS/H2',chrome_tls_exception='test certificate SPKI only',firefox_tls_exception='disposable session acceptInsecureCerts',normal_profiles_modified=False),indent=2)+'\n')
        # Probe the production client while the controlled endpoints still exist.
        subprocess.run(['cargo','run','-p','netscope','--example','check_campaign','--offline','--',str(root),'--live'],cwd=script.parents[1],check=True,timeout=240)
finally:
    for proc in reversed(processes):
        proc.terminate()
        try:proc.wait(timeout=10)
        except subprocess.TimeoutExpired:proc.kill();proc.wait()
    import shutil
    for name in ['chrome-profile','ff-marionette']:
        shutil.rmtree(root/name,ignore_errors=True)
