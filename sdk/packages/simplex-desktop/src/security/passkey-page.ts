const style = `
:root{color-scheme:dark;--bg:#131417;--card:oklch(0.243 0.0123 264.31);--raised:oklch(0.2593 0.0144 261.67);--text:#fff;--muted:#929daa;--border:rgba(255,255,255,.08);--strong:rgba(255,255,255,.14);--ok:#74e998;--err:#ff7a89;--focus:#4adeff;--ease:cubic-bezier(.23,1,.32,1)}
*{box-sizing:border-box}
html{background:var(--bg)}
body{display:grid;min-height:100dvh;place-items:center;margin:0;padding:2rem 1rem;background:radial-gradient(circle at 12% 0%,rgba(74,222,255,.055),transparent 28rem),radial-gradient(circle at 88% 4%,rgba(255,53,235,.04),transparent 30rem),var(--bg);color:var(--text);font:15px/1.55 system-ui,-apple-system,"Segoe UI",sans-serif;-webkit-font-smoothing:antialiased}
main{display:grid;gap:1.25rem;width:min(100%,26rem);padding:2.25rem;border:1px solid var(--border);border-radius:1.25rem;background:var(--card);box-shadow:0 24px 64px rgba(0,0,0,.35)}
.icon{display:grid;width:3rem;height:3rem;place-items:center;border:1px solid var(--strong);border-radius:.9rem;background:var(--raised);color:var(--text);transition:color 180ms var(--ease),border-color 180ms var(--ease)}
.icon svg{width:1.5rem;height:1.5rem}
.icon .done,.icon .fail,[data-state=done] .icon .key,[data-state=fail] .icon .key{display:none}
[data-state=done] .icon{border-color:rgba(116,233,152,.35);color:var(--ok)}
[data-state=done] .icon .done{display:block}
[data-state=fail] .icon{border-color:rgba(255,122,137,.35);color:var(--err)}
[data-state=fail] .icon .fail{display:block}
.eyebrow{margin:0 0 .5rem;color:var(--muted);font:500 11px ui-monospace,monospace;letter-spacing:.1em;text-transform:uppercase}
h1{margin:0;font-size:1.6rem;line-height:1.2;letter-spacing:-.03em}
p{margin:0}
#status{color:var(--muted)}
.actions{display:grid;gap:.5rem}
[data-state=done] .actions,[data-state=fail] .actions{display:none}
button{min-height:2.75rem;padding:.65rem 1rem;border:1px solid var(--border);border-radius:.75rem;background:transparent;color:var(--muted);font:500 14px system-ui,-apple-system,"Segoe UI",sans-serif;cursor:pointer;transition:transform 120ms var(--ease),background-color 180ms var(--ease),color 180ms var(--ease)}
button:hover:not(:disabled){background:var(--raised);color:var(--text)}
button:active:not(:disabled){transform:scale(.97)}
button:focus-visible{outline:2px solid var(--focus);outline-offset:2px}
button:disabled{cursor:not-allowed;opacity:.5}
#continue{border-color:#fff;background:#fff;color:var(--bg)}
#continue:hover:not(:disabled){background:#f4f4f4;color:var(--bg)}
.foot{padding-top:1rem;border-top:1px solid var(--border);color:var(--muted);font-size:12px}
@media (prefers-reduced-motion:reduce){*{transition:none!important}}
`

const icons = `<svg class="key" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="8" cy="15" r="4"/><path d="m10.85 12.15 8.65-8.65M18 5l2 2M15 8l2 2"/></svg>
<svg class="done" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>
<svg class="fail" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 7.5v5.5M12 16.5v.01"/><circle cx="12" cy="12" r="9"/></svg>`

/** Served only by the temporary loopback ceremony server; no config keys reach this page. */
export function passkeyPage(nonce: string): string {
	return `<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Simplex passkey</title>
<style nonce="${nonce}">${style}</style>
<main aria-labelledby="title">
<div class="icon">${icons}</div>
<div><p class="eyebrow">Simplex Desktop</p><h1 id="title">Confirm your passkey</h1></div>
<p id="status" role="status">Simplex asked this browser to confirm your passkey.</p>
<div class="actions"><button id="continue" disabled>Continue</button><button id="cancel">Cancel</button></div>
<p class="foot">This page only talks to Simplex on this computer. Nothing is sent to a website.</p>
</main>
<script nonce="${nonce}">
let token = location.hash.slice(1);
// Keep the capability for a reload of this tab only; the address bar never shows it again.
try {
  if (token) sessionStorage.setItem('token', token);
  else token = sessionStorage.getItem('token') || '';
} catch {}
history.replaceState(null, '', '/');
const main = document.querySelector('main');
const title = document.getElementById('title');
const status = document.getElementById('status');
const proceed = document.getElementById('continue');
const cancel = document.getElementById('cancel');
const decode = value => Uint8Array.from(atob(value.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0));
const encode = value => btoa(String.fromCharCode(...new Uint8Array(value))).replace(/\\+/g, '-').replace(/\\//g, '_').replace(/=+$/, '');
const controller = new AbortController();
const copy = {
  register: ['Create your passkey', 'Save a passkey to sign in with Touch ID or Windows Hello.', 'Create passkey'],
  authenticate: ['Unlock Simplex', 'Use your passkey to continue.', 'Unlock with passkey'],
};
let kind;
let options;
function show(state, heading, text) {
  main.dataset.state = state;
  title.textContent = heading;
  status.textContent = text;
  proceed.disabled = cancel.disabled = true;
}
function failure(heading, text) {
  return Object.assign(new Error(text), {heading});
}
async function request(path, body) {
  let response;
  try {
    response = await fetch(path, {method:'POST', headers:{'Content-Type':'application/json', 'Authorization':'Bearer '+token}, body:JSON.stringify(body || {})});
  } catch {
    throw failure('Can\\'t reach Simplex', 'Make sure Simplex is still open, then start again from the app.');
  }
  if (!response.ok) throw path === '/verify'
    ? failure('Something went wrong', 'Simplex couldn\\'t verify this passkey. Return to Simplex to try again.')
    : failure('Request ended', 'This passkey request is no longer active. Return to Simplex to start a new one.');
  return response.json();
}
// Load options before the click so the browser still sees the click as the user gesture.
request('/options').then(result => {
  if (main.dataset.state) return; // Cancelled while loading.
  ({kind, options} = result);
  const [heading, text, action] = copy[kind];
  document.title = heading + ' · Simplex';
  title.textContent = heading;
  status.textContent = text;
  proceed.textContent = action;
  proceed.disabled = false;
  proceed.focus();
}, error => {
  if (main.dataset.state) return;
  show('fail', error.heading, error.message);
  // Release Simplex now rather than leaving it waiting for the timeout.
  request('/cancel').catch(() => {});
});
proceed.onclick = async () => {
  proceed.disabled = true;
  try {
    if (!window.PublicKeyCredential || !await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable())
      throw new Error('This browser can\\'t use a device passkey. Set up Touch ID or Windows Hello, then try again from Simplex.');
    const publicKey = {...options, challenge:decode(options.challenge)};
    if (publicKey.user) publicKey.user = {...publicKey.user, id:decode(publicKey.user.id)};
    for (const key of ['allowCredentials', 'excludeCredentials'])
      if (publicKey[key]) publicKey[key] = publicKey[key].map(item => ({...item, id:decode(item.id)}));
    const credential = await navigator.credentials[kind === 'register' ? 'create' : 'get']({publicKey, signal:controller.signal});
    const r = credential.response;
    const response = {clientDataJSON:encode(r.clientDataJSON)};
    if (kind === 'register') {
      response.attestationObject = encode(r.attestationObject);
      response.transports = r.getTransports ? r.getTransports() : [];
    } else {
      response.authenticatorData = encode(r.authenticatorData);
      response.signature = encode(r.signature);
      response.userHandle = r.userHandle ? encode(r.userHandle) : undefined;
    }
    await request('/verify', {id:credential.id, rawId:encode(credential.rawId), type:credential.type,
      authenticatorAttachment:credential.authenticatorAttachment, clientExtensionResults:credential.getClientExtensionResults(), response});
    show('done', kind === 'register' ? 'Passkey created' : 'Passkey verified',
      'You\\'re all set. Simplex is continuing in its own window, so you can close this tab.');
  } catch (error) {
    const cancelled = error.name === 'NotAllowedError' || error.name === 'AbortError';
    show('fail', cancelled ? 'Request cancelled' : error.heading || 'Something went wrong', cancelled
      ? 'Passkey request cancelled. Nothing changed. Return to Simplex to try again.' : error.message);
    await request('/cancel').catch(() => {});
  }
};
cancel.onclick = async () => {
  controller.abort();
  show('fail', 'Request cancelled', 'Cancelled. Nothing changed. You can close this tab and return to Simplex.');
  await request('/cancel').catch(() => {});
};
</script></html>`
}
