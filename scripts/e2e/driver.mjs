// Minimal WebDriver client for driving the real Duragon window through tauri-driver.
// Usage: see scripts/e2e/README.md

const W3C_ELEMENT = 'element-6066-11e4-a6c5-4ef1-a1e1-ca6d1fc2f5a5';

export class Driver {
  constructor(base = 'http://127.0.0.1:4444') {
    this.base = base;
    this.session = null;
  }

  async req(method, path, body) {
    const res = await fetch(`${this.base}${path}`, {
      method,
      headers: { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const json = await res.json();
    if (!res.ok || json.value?.error) {
      throw new Error(`${method} ${path}: ${json.value?.error} ${json.value?.message ?? ''}`);
    }
    return json.value;
  }

  async start(application) {
    const v = await this.req('POST', '/session', {
      capabilities: { alwaysMatch: { 'tauri:options': { application } } },
    });
    this.session = v.sessionId;
    return this;
  }

  s(path) {
    return `/session/${this.session}${path}`;
  }

  /** Run JS in the page. The body sees `args`; return a JSON-serialisable value. */
  exec(body, ...args) {
    return this.req('POST', this.s('/execute/sync'), { script: body, args });
  }

  /** Run async JS: the body must call `done(value)`. */
  execAsync(body, ...args) {
    const script = `const done = arguments[arguments.length - 1]; const args = [...arguments].slice(0, -1); ${body}`;
    return this.req('POST', this.s('/execute/async'), { script, args });
  }

  async find(css) {
    const v = await this.req('POST', this.s('/element'), { using: 'css selector', value: css });
    return v[W3C_ELEMENT] ?? v.ELEMENT ?? Object.values(v)[0];
  }

  async click(css) {
    const id = await this.find(css);
    await this.req('POST', this.s(`/element/${id}/click`), {});
  }

  async type(css, text) {
    const id = await this.find(css);
    await this.req('POST', this.s(`/element/${id}/value`), { text });
  }

  async screenshot(path) {
    const b64 = await this.req('GET', this.s('/screenshot'));
    const { writeFile } = await import('node:fs/promises');
    await writeFile(path, Buffer.from(b64, 'base64'));
    return path;
  }

  /** Poll `predicate` (a JS expression returning truthy) until it holds or the timeout passes. */
  async waitFor(expr, timeoutMs = 60000, everyMs = 500) {
    const end = Date.now() + timeoutMs;
    for (;;) {
      const v = await this.exec(`return (${expr});`);
      if (v) return v;
      if (Date.now() > end) throw new Error(`timed out waiting for: ${expr}`);
      await new Promise((r) => setTimeout(r, everyMs));
    }
  }

  async quit() {
    if (this.session) await this.req('DELETE', this.s('')).catch(() => {});
    this.session = null;
  }
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
