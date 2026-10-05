// Exercise the built host over HTTP without logging account/session credentials.
const base = process.env.BQATLAS_IMAGE_TEST_ORIGIN;
if (!base || !/^http:\/\/127\.0\.0\.1:\d+$/.test(base)) throw new Error('A loopback image test origin is required');
const cookies = new Map();
async function call(path, options = {}) {
  const response = await fetch(base + path, { ...options, headers: {
    Cookie: [...cookies].map(([key, value]) => `${key}=${value}`).join('; '),
    ...options.headers,
  }, redirect: 'manual' });
  for (const cookie of response.headers.getSetCookie()) {
    const pair = cookie.split(';')[0];
    const split = pair.indexOf('=');
    cookies.set(pair.slice(0, split), pair.slice(split + 1));
  }
  return response;
}
function expect(value, message) { if (!value) throw new Error(message); }
for (const path of ['/health/live', '/health/ready']) {
  const response = await call(path);
  expect(response.status === 200, `${path} unavailable`);
}
const index = await call('/');
expect(index.status === 200, 'Frontend unavailable');
const html = await index.text();
expect(html.includes('bqAtlas Rust CRM'), 'Wrong frontend served');
const script = html.match(/<script[^>]*src="([^"<>]+)"/);
expect(script && !script[1].includes('://') && !script[1].startsWith('//'), 'No local script asset');
const asset = await call('/' + script[1].replace(/^\//, ''));
expect(asset.status === 200 && (asset.headers.get('content-type') || '').includes('javascript'), 'Frontend asset unavailable');
for (const path of ['/api/v1/unknown', '/odata/Unknown', '/auth/unknown']) {
  const response = await call(path);
  expect(response.status === 404 && (response.headers.get('content-type') || '').includes('application/problem+json'), `SPA masked ${path}`);
}
let response = await call('/api/v1/session/csrf');
expect(response.status === 200, 'CSRF unavailable');
const sessionCookie = response.headers.getSetCookie().find(value => value.startsWith('bqatlas-rust.session='));
expect(sessionCookie && /; Secure/i.test(sessionCookie) && /; HttpOnly/i.test(sessionCookie), 'Production cookie flags missing');
let csrf = (await response.json()).token;
response = await call('/auth/login', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-BQATLAS-CSRF': csrf }, body: JSON.stringify({email: process.env.BQATLAS_IMAGE_TEST_EMAIL, password: process.env.BQATLAS_IMAGE_TEST_PASSWORD}) });
expect(response.status === 204, 'Image account login failed');
response = await call('/api/v1/session');
expect(response.status === 200 && (await response.json()).authenticated === true, 'Image session not authenticated');
csrf = (await (await call('/api/v1/session/csrf')).json()).token;
response = await call('/api/v1/crm/customers/query', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-BQATLAS-CSRF': csrf }, body: JSON.stringify({page:0,pageSize:25}) });
expect(response.status === 200 && (await response.json()).total === 4, 'Image CRM query failed');
response = await call('/api/v1/crm/customers/not-a-uuid');
expect(response.status === 404 && response.headers.get('cache-control') === 'no-store', 'Image record rejection differs');
console.log('Production image passed static assets, health, secure cookie/session login, seeded CRM and reserved-path checks.');
