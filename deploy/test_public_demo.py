"""HTTP isolation checks using dummy keys; never calls an external LLM."""
import json
import sys
import urllib.error
import urllib.request

base = sys.argv[1].rstrip('/')

def request(path, body=None, cookie=None, secure=False, session=None):
    headers = {'Content-Type': 'application/json'}
    if cookie:
        headers['Cookie'] = cookie
    if secure:
        headers['X-Forwarded-Proto'] = 'https'
    if session is not None:
        headers['X-Selmem-Session'] = session
    req = urllib.request.Request(base + path, headers=headers,
        data=json.dumps(body).encode() if body is not None else None)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return response.status, response.headers, response.read().decode()
    except urllib.error.HTTPError as response:
        return response.code, response.headers, response.read().decode()

def api(path, body=None, cookie=None):
    status, _, text = request(path, body, cookie)
    assert status == 200, (path, status)  # Never print bodies that might hold secrets.
    return json.loads(text)

a_status, a_headers, a_text = request('/session', secure=True)
b_status, b_headers, b_text = request('/session')
assert a_status == b_status == 200
assert 'HttpOnly' in a_headers['Set-Cookie']
assert 'SameSite=Strict' in a_headers['Set-Cookie']
assert 'Secure' in a_headers['Set-Cookie']
a = a_headers['Set-Cookie'].split(';')[0]
b = b_headers['Set-Cookie'].split(';')[0]
assert a != b
a_state, b_state = json.loads(a_text), json.loads(b_text)
assert a_state['memory_lifetime_hours'] == 24
assert a_state['session_id'] != b_state['session_id']
assert json.loads(request('/session', session=a_state['session_id'])[2])['session_id'] == a_state['session_id']
assert request('/stats', session='forged')[0] == 401
assert api('/health') == {'ok': True, 'public_demo': True}
assert request('/stats')[0] == 401
assert request('/stats', cookie='selmem_session=forged')[0] == 401
assert request('/profile')[0] == 401
assert request('/profile', cookie='selmem_session=forged')[0] == 401
assert request('/live', {'event': 'unauthenticated write'})[0] == 401
assert 'Demo' in request('/')[2] or '<html' in request('/')[2]
assert api('/session', cookie=a)['public_demo'] is True
api('/profile', {'name': 'visitor-A'}, a)
assert api('/profile', cookie=a)['name'] == 'visitor-A'
assert api('/profile', cookie=b)['name'] != 'visitor-A'
assert api('/live', {'event': 'The appointment is Tuesday in room B.', 'channel': 'world'}, a)['kept']
assert len(api('/book', cookie=a)['traces']) == 1
assert len(api('/book', cookie=b)['traces']) == 0
stats_a = api('/stats', cookie=a)
stats_b = api('/stats', cookie=b)
assert stats_a['name'] == 'visitor-A'
assert stats_b['name'] != 'visitor-A'
assert stats_a['traces'] == 1
assert stats_b['traces'] == 0
for stats, cookie in ((stats_a, a), (stats_b, b)):
    book = api('/book', cookie=cookie)
    for counter in ('traces', 'axioms', 'archives'):
        assert stats[counter] == len(book[counter])
assert api('/health', cookie=a) == {'ok': True, 'public_demo': True}
api('/profile', {'time_scale': 150}, a)
assert api('/profile', cookie=a)['time_scale'] == 150
assert api('/profile', cookie=b)['time_scale'] == 24
sleep_a = api('/sleep', {}, a)
sleep_b = api('/sleep', {}, b)
assert sleep_a['hours_advanced'] == sleep_a['time_scale'] == 150
assert sleep_b['hours_advanced'] == sleep_b['time_scale'] == 24
assert api('/llm', cookie=a)['has_key'] is False
assert request('/turn', {'text': 'Hello'}, a)[0] == 400
assert all(not plug['has_key'] for plug in api('/llm', cookie=a)['plugs'])
api('/llm', {'plug': 'gpt'}, a)
assert api('/llm', cookie=a)['has_key'] is False
assert request('/turn', {'text': 'Hello'}, a)[0] == 400
key_a = 'dummy-visitor-A-key-never-used'
key_b = 'dummy-visitor-B-key-never-used'
api('/llm', {'plug': 'gpt', 'api_key': key_a}, a)
assert api('/llm', cookie=a)['has_key'] is True
assert api('/llm', cookie=b)['has_key'] is False
assert key_a not in request('/llm', cookie=a)[2]
assert key_a not in request('/book', cookie=a)[2]
assert json.loads(request('/stats', cookie=b, session=a_state['session_id'])[2])['name'] == 'visitor-A'
api('/llm', {'plug': 'grok'}, b)
api('/llm', {'api_key': key_b}, b)
assert api('/llm', cookie=b)['has_key'] is True
assert api('/llm', cookie=a)['plug'] == 'gpt'
api('/llm', {'plug': 'grok'}, a)
assert api('/llm', cookie=a)['has_key'] is False
assert api('/llm', cookie=b)['has_key'] is True
assert request('/llm', {'url': 'http://127.0.0.1/private'}, a)[0] == 400
api('/llm', {'clear': True}, a)
assert not api('/llm', cookie=a)['attached']
assert api('/llm', cookie=b)['has_key'] is True
assert request('/turn', {'text': 'Hello'}, a)[0] == 400
assert len(api('/book', cookie=b)['traces']) == 0
print('Public demo passed: browser session resume, isolated stats/books/profiles/keys, no server-key inheritance, provider switch, endpoint restriction, LLM setup required.')
