#!/usr/bin/env python3
"""Validate the crates.io publication age of every registry entry in Cargo.lock.
Uses Cargo's sparse-index metadata. Never downloads or executes package sources.
"""
from pathlib import Path
import datetime,json,sys
import tomllib
root=Path(__file__).resolve().parents[1]
lock=tomllib.loads((root/'Cargo.lock').read_text())
cut=datetime.datetime.now(datetime.timezone.utc)-datetime.timedelta(days=7)
indices=list((Path.home()/'.cargo/registry/index').glob('*/.cache'))
failures=[];checked=0
for package in lock['package']:
 if not package.get('source','').startswith('registry+'):continue
 name=package['name'];suffix=name if len(name)==1 else '2/'+name if len(name)==2 else '3/'+name[0]+'/'+name if len(name)==3 else name[:2]+'/'+name[2:4]+'/'+name
 record=None
 for index in indices:
  file=index/suffix
  if not file.exists():continue
  for chunk in file.read_bytes().split(b'\0'):
   if chunk.startswith(b'{'):
    item=json.loads(chunk)
    if item['vers']==package['version']:record=item;break
  if record:break
 if not record or not record.get('pubtime'):failures.append({'name':name,'version':package['version'],'reason':'NOT REVIEWED: no publication metadata'});continue
 published=datetime.datetime.fromisoformat(record['pubtime'].replace('Z','+00:00'))
 if record.get('yanked') or published>cut:failures.append({'name':name,'version':package['version'],'published':record['pubtime'],'reason':'yanked or less than 7 days old'})
 else:checked+=1
result={'checked':checked,'cutoff':cut.isoformat(),'failures':failures}
if '--check' not in sys.argv[1:]:
 (root/'docs/dependency-release-age.json').write_text(json.dumps(result,indent=2))
print(json.dumps(result,indent=2));sys.exit(bool(failures) or checked==0)
