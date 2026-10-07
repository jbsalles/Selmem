// Run the embedded browser code against the real daemon without contacting an LLM.
// A minimal DOM supplies event targets; fetch and localStorage follow browser usage.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const base = process.argv[2];
const html = fs.readFileSync(new URL('../src/net/ui.html', import.meta.url), 'utf8');
const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
assert.match(html, /<option value="grok">Grok<\/option>/);
assert.match(html, /<option value="gpt">Gpt<\/option>/);
assert.doesNotMatch(html, /value="rules"/);
assert.match(html, /Memory is cleared every 24 hours/);
function classes() {
  const values = new Set();
  return { contains: k => values.has(k), remove: k => values.delete(k),
    add: k => values.add(k), toggle(k, on) {
      const active = on ?? !values.has(k);
      if (active) values.add(k); else values.delete(k);
      return active;
    } };
}
async function browser(storage = new Map()) {
  const elements = new Map();
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      value: id === 'llm-plug' ? 'grok' : '', step: '0.01', textContent: '',
      hidden: false, classList: classes(), messages: [], dataset: {},
      addEventListener(event, fn) { this[event] = fn; },
      closest() { return element(id + '-fieldset'); },
      querySelector() { return element(id + '-hint'); }, focus() {},
      appendChild(child) { this.messages.push(child); },
      get innerHTML() { return this.messages.map(m => m.innerHTML).join(''); },
      set innerHTML(value) { this.messages = value ? [{ innerHTML: value }] : []; }
    });
    return elements.get(id);
  }
  const calls = [];
  const context = vm.createContext({
    document: { getElementById: element, createElement: () => ({ innerHTML: '' }),
      body: { classList: classes() }, querySelectorAll: () => [],
      querySelector: selector => element(selector) },
    localStorage: { getItem: k => storage.get(k) ?? null, setItem: (k, v) => storage.set(k, String(v)) },
    fetch: async (path, options) => {
      calls.push({ path, ...options });
      try {
        return await fetch(base + path, { ...options, signal: AbortSignal.timeout(10000) });
      } catch (cause) { throw new Error(`HTTP ${options.method} ${path} failed`, { cause }); }
    }, setTimeout, clearTimeout, console
  });
  vm.runInContext(script, context);
  const run = code => vm.runInContext(code, context);
  await run('sessionReady');
  await run('refresh()');
  return { storage, element, calls, run };
}
const a = await browser();
const id = a.storage.get('selmem_session');
assert.match(id, /^[0-9a-f]{64}$/);
assert.equal(a.element('name').textContent, 'Claire');
assert.equal(a.element('memory-warning').hidden, false);
a.element('text').value = 'Hello';
await a.element('f').onsubmit({ preventDefault() {} });
assert.match(a.element('log').innerHTML, /No, LLM setup, please configure it in options/);
assert.equal(a.calls.filter(c => c.path === '/turn').length, 0);
await a.run('fillOpts()');
await a.run("api('POST', '/live', {event:'The appointment is Tuesday in room B.', channel:'world'})");
a.element('llm-plug').value = 'grok';
a.element('llm-key').value = 'dummy-browser-A-grok';
await a.element('llm-save').onclick();
a.element('opt-name').value = 'Alice';
a.element('encode_threshold').value = '0.45';
a.element('time_scale').value = '150';
a.element('cut_ladder').checked = false;
await a.run('saveOpts()');
const returning = await browser(a.storage);
assert.equal(returning.storage.get('selmem_session'), id);
assert.equal(returning.element('name').textContent, 'Alice');
assert.equal((await returning.run("api('GET', '/llm')")).has_key, true);
assert.equal((await returning.run("api('GET', '/book')")).traces.length, 1);
await returning.run('fillLlm()');
assert.equal(returning.element('llm-key').value, 'dummy-browser-A-grok');
returning.element('llm-plug').value = 'gpt';
await returning.element('llm-plug').change();
assert.equal(returning.element('llm-key').value, '');
returning.element('llm-key').value = 'dummy-browser-A-gpt';
await returning.element('llm-save').onclick();
await returning.element('llm-clear').onclick();
assert.equal((await returning.run("api('GET', '/llm')")).has_key, false);
assert.equal(JSON.parse(a.storage.get('selmem_settings')).llm.keys.gpt, '');
returning.element('llm-plug').value = 'grok';
await returning.element('llm-plug').change();
assert.equal(returning.element('llm-key').value, 'dummy-browser-A-grok');
assert.equal((await returning.run("api('GET', '/llm')")).has_key, true);
const b = await browser();
assert.notEqual(b.storage.get('selmem_session'), id);
assert.equal(b.element('name').textContent, 'Claire');
assert.equal((await b.run("api('GET', '/llm')")).has_key, false);
assert.equal((await b.run("api('GET', '/book')")).traces.length, 0);
assert.equal((await b.run("api('GET', '/profile')")).time_scale, 24);
// Simulate the session-expired HTTP path: invalid IDs must produce fresh spaces,
// not resurrect old memory. The same restoration is used after daily expiry/restarts.
a.storage.set('selmem_session', 'expired-browser-id');
returning.run("sessionId = 'expired-browser-id'");
assert.equal((await returning.run("api('GET', '/profile')")).name, 'Alice');
const restoredProfile = await returning.run("api('GET', '/profile')");
assert.equal(restoredProfile.encode_threshold, 0.45);
assert.equal(restoredProfile.time_scale, 150);
assert.equal(restoredProfile.cut_ladder, false);
assert.notEqual(a.storage.get('selmem_session'), id);
assert.equal((await returning.run("api('GET', '/book')")).traces.length, 0);
assert.equal((await returning.run("api('GET', '/llm')")).has_key, true);
assert.match(returning.element('log').innerHTML, /temporary memory was cleared/);
assert.equal((await b.run("api('GET', '/llm')")).has_key, false);
console.log('UI passed: Claire, required LLM setup, saved keys/options, reload, separate browser spaces, provider switching, forget key, expired-session restoration.');
