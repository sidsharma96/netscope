"""120 distinct controlled exchanges; bodies are synthetic, captures are native."""
import base64
import gzip
import hashlib
import json


def cases():
    result = []
    # Each entry below changes protocol/application behavior rather than just its ID.
    rest = [
        ('get-json', {}), ('get-text', dict(mime='text/plain', body='books')),
        ('unicode-json', dict(body='{"title":"café 📚"}')),
        ('empty-json-list', dict(body='[]')), ('nested-json', dict(body='{"a":{"b":[1,null,true]}}')),
        ('repeated-query', dict(query='tag=a&tag=b&empty=')),
        ('escaped-query', dict(query='q=caf%C3%A9%20%26%20books')),
        ('json-post', dict(method='POST', request='{"title":"café"}', request_mime='application/json')),
        ('empty-post', dict(method='POST', request='')),
        ('form-post', dict(method='POST', request='tag=a&tag=b&empty=', request_mime='application/x-www-form-urlencoded')),
        ('text-post', dict(method='POST', request='first\nsecond\tline', request_mime='text/plain')),
        ('put-json', dict(method='PUT', request='{"replace":true}', request_mime='application/json')),
        ('patch-json', dict(method='PATCH', request='{"title":"new"}', request_mime='application/json')),
        ('delete', dict(method='DELETE')), ('head', dict(method='HEAD')),
        ('options', dict(method='OPTIONS')), ('binary-response', dict(mime='application/octet-stream', binary=[0,255,1,254])),
        ('empty-text', dict(mime='text/plain', body='')), ('html', dict(mime='text/html', body='<p>books &amp; café</p>')),
        ('csv', dict(mime='text/csv', body='id,title\n1,book\n')),
        ('xml', dict(mime='application/xml', body='<book>café</book>')),
        ('gzip-text', dict(mime='text/plain', body='books '*300, gzip=True)),
        ('gzip-json', dict(gzip=True)), ('chunked-text', dict(mime='text/plain', body='first second', chunked=True)),
        ('chunked-unicode', dict(mime='text/plain', body='café 📚', chunked=True, export_text={'chrome':'cafÃ© ðŸ“š'})),
        ('cookie-set', dict(cookie=True)), ('duplicate-response-header', dict(duplicate=True)),
        ('custom-request-header', dict(header='X-Campaign')),
        ('large-json', dict(body=json.dumps(dict(books=['café']*1000), ensure_ascii=False))),
        ('unicode-path-query', dict(query='name=%F0%9F%93%9A')),
    ]
    for status in [201,202,204,206,304,400,401,403,404,405,409,410,415,422,429,500,502,503,504]:
        rest.append((f'status-{status}', dict(status=status, body='' if status in (204,304) else json.dumps(dict(status=status)))))
    rest.append(('redirect-follow', dict(status=302, follow=True)))
    for name, config in rest:
        # Merge defaults explicitly so overridden fields are never duplicate kwargs.
        result.append(dict(id=f'R{len([c for c in result if c["family"]=="R"])+1:02}', family='R', name=name,
                           **(dict(method='GET', status=200, mime='application/json', body='{"ok":true}') | config)))
    graphql = [
        ('query', '{ books { id title } }', dict(data={'books':[{'id':'1','title':'café'}]})),
        ('named-query', 'query Books { books { id } }', dict(data={'books':[{'id':'1'}]})),
        ('variables', 'query Book($id: ID!) { book(id:$id) { title } }', dict(data={'book':{'title':'one'}})),
        ('mutation', 'mutation { addBook(title:"new") { id } }', dict(data={'addBook':{'id':'2'}})),
        ('aliases', '{ first: book(id:"1") { id } second: book(id:"2") { id } }', dict(data={'first':{'id':'1'},'second':{'id':'2'}})),
        ('fragment', 'query { books { ...B } } fragment B on Book { id title }', dict(data={'books':[]})),
        ('empty-result', '{ books { id } }', dict(data={'books':[]})),
        ('null-result', '{ book(id:"absent") { id } }', dict(data={'book':None})),
        ('validation-error', '{ nonexistent }', dict(errors=[dict(message='Unknown field nonexistent')])),
        ('syntax-error', '{', dict(errors=[dict(message='Syntax error')])),
        ('partial-error', '{ books { title } }', dict(data={'books':[None]}, errors=[dict(message='Unavailable',path=['books',0])])),
        ('extensions', '{ books { id } }', dict(data={'books':[]}, extensions={'cost':3})),
        ('unicode-result', '{ book { title } }', dict(data={'book':{'title':'日本語 📚'}})),
        ('large-result', '{ books { title } }', dict(data={'books':[{'title':'book'}]*500})),
        ('get-query', '{ books { id } }', dict(data={'books':[]})),
        ('operation-name', 'query A { books { id } } query B { book { id } }', dict(data={'book':{'id':'1'}})),
        ('nullable-variables', 'query($id:ID) { book(id:$id) { id } }', dict(data={'book':None})),
        ('directives', '{ books @include(if:true) { id } }', dict(data={'books':[]})),
        ('introspection-shape', '{ __typename }', dict(data={'__typename':'Query'})),
        ('persisted-query-shape', '', dict(errors=[dict(message='PersistedQueryNotFound')])),
        ('batch-shape', '', [dict(data={'books':[]}),dict(data={'book':None})]),
        ('auth-error', '{ books { id } }', dict(errors=[dict(message='Unauthorized')])),
        ('rate-limit-error', '{ books { id } }', dict(errors=[dict(message='Rate limited')])),
        ('graphql-mime', '{ books { id } }', dict(data={'books':[]})),
        ('compressed-result', '{ books { id } }', dict(data={'books':[]})),
    ]
    for name, query, response in graphql:
        request=json.dumps(dict(query=query,variables={'id':'1'},operationName='B' if name=='operation-name' else None),ensure_ascii=False)
        if name=='batch-shape': request=json.dumps([dict(query='{ books { id } }'),dict(query='{ book { id } }')])
        if name=='persisted-query-shape':request=json.dumps(dict(extensions={'persistedQuery':{'version':1,'sha256Hash':'0'*64}}))
        config=dict(method='GET' if name=='get-query' else 'POST',status=401 if name=='auth-error' else 429 if name=='rate-limit-error' else 400 if name=='syntax-error' else 200,
                    mime='application/graphql-response+json' if name=='graphql-mime' else 'application/json',
                    body=json.dumps(response,ensure_ascii=False),request=request,request_mime='application/json')
        if name=='get-query':config['query']='query=%7B%20books%20%7B%20id%20%7D%20%7D';config.pop('request')
        if name=='compressed-result':config['gzip']=True
        result.append(dict(id=f'G{sum(c["family"]=="G" for c in result)+1:02}',family='G',name=name,**config))
    events = [
        ('basic','data: one\n\n'), ('multiline','data: one\ndata: two\n\n'),
        ('unicode','data: café 📚\n\n'), ('empty-data','data:\n\n'),
        ('event-type','event: added\ndata: book\n\n'), ('id-inherit','id: book-1\ndata: one\n\ndata: two\n\n'),
        ('id-reset','id: x\ndata: one\n\nid:\ndata: two\n\n'), ('retry','retry: 1500\ndata: one\n\n'),
        ('huge-retry','retry: 99999999999999999999999999999999999\ndata: one\n\n'),
        ('comments',': keepalive\n\ndata: one\n\n'), ('unknown-field','unknown: x\ndata: one\n\n'),
        ('literal-case','Data: ignored\ndata: yes\n\n'), ('one-space','data:  leading\n\n'),
        ('no-colon','data\n\n'), ('metadata-only','id: a\nevent: added\n\ndata: one\n\n'),
        ('lf','data: one\n\n'), ('crlf','data: one\r\n\r\n'), ('cr','data: one\r\r'),
        ('bom','\ufeffdata: one\n\n'), ('unfinished','data: complete\n\ndata: pending\n'),
        ('nul-id','id: x\0y\ndata: one\n\n'), ('hundred-events','data: x\n\n'*100),
        ('hundred-one-events','data: x\n\n'*101), ('text-budget','data: '+ 'x'*66000+'\n\n'),
        ('compressed','data: gzip event\n\n'),
    ]
    for name,body in events:
        result.append(dict(id=f'S{sum(c["family"]=="S" for c in result)+1:02}',family='S',name=name,method='GET',status=200,mime='text/event-stream; charset=utf-8',body=body,gzip=name=='compressed'))
    names=['text','unicode','empty-text','two-messages','binary','fragmented','ping-text','subprotocol-chat','graphql-transport-ws','json-message','long-text','close-normal','close-going-away','reject-403','reject-404','missing-accept','bad-digest','missing-upgrade','unoffered-protocol','connection-token-case']
    for name in names:
        result.append(dict(id=f'W{sum(c["family"]=="W" for c in result)+1:02}',family='W',name=name,method='GET',status=403 if name=='reject-403' else 404 if name=='reject-404' else 101,mime='',body=''))
    assert len(result)==120 and len({c['name']+c['family'] for c in result})==120
    for c in result:
        raw=bytes(c['binary']) if 'binary' in c else c['body'].encode()
        c['decoded_bytes']=len(raw)
        c['decoded_sha256']=hashlib.sha256(raw).hexdigest()
        c['wire_base64']=base64.b64encode(gzip.compress(raw,mtime=0) if c.get('gzip') else raw).decode()
        c['tls']=c['family'] in ('R','G') and int(c['id'][1:])%3==0 and not c.get('chunked') and not c.get('follow')
        if c['id'] in ['S01','S02','S04','S05','S10','W01','W02','W05','W08','W09','W20']:c['tls']=True
    return result
