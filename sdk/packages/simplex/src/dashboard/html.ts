export interface DashboardConfig {
	fillerAddress: string
	chains: number[]
	strategies: Array<
		| {
				type: "basic"
				bpsCurve: Array<{ amount: string; value: number }>
		  }
		| {
				type: "hyperfx"
				bidPriceCurve: Array<{ amount: string; price: string }>
				askPriceCurve: Array<{ amount: string; price: string }>
				maxOrderUsd: string
				exoticTokenAddresses: Record<string, string>
		  }
	>
}

export function getDashboardHtml(config: DashboardConfig): string {
	const configJson = JSON.stringify(config)

	return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Simplex Dashboard</title>
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.3/dist/chart.umd.min.js"></script>
<style>
  body { font-family: 'Courier New', Courier, monospace; }
  .stat-card { background: #0f172a; border: 1px solid #1e293b; border-radius: 8px; padding: 20px; }
  .panel { background: #0f172a; border: 1px solid #1e293b; border-radius: 8px; }
  .activity-item { border-bottom: 1px solid #1e293b; }
  .activity-item:last-child { border-bottom: none; }
  @keyframes pulse-dot { 0%,100%{opacity:1} 50%{opacity:0.3} }
  .pulse { animation: pulse-dot 2s infinite; }
  .hash { font-size: 11px; color: #64748b; }
  .badge-success { background: #064e3b; color: #10b981; border-radius: 4px; padding: 2px 6px; font-size: 11px; }
  .badge-fail { background: #450a0a; color: #ef4444; border-radius: 4px; padding: 2px 6px; font-size: 11px; }
  .badge-retracted { background: #1e1b4b; color: #818cf8; border-radius: 4px; padding: 2px 6px; font-size: 11px; }
  .badge-pending { background: #1c1917; color: #f59e0b; border-radius: 4px; padding: 2px 6px; font-size: 11px; }
  .tag-order { background: #0c4a6e; color: #38bdf8; border-radius: 4px; padding: 1px 5px; font-size: 10px; }
  .tag-fill { background: #064e3b; color: #10b981; border-radius: 4px; padding: 1px 5px; font-size: 10px; }
  .tag-skip { background: #451a03; color: #fb923c; border-radius: 4px; padding: 1px 5px; font-size: 10px; }
  .tag-exec { background: #1e1b4b; color: #818cf8; border-radius: 4px; padding: 1px 5px; font-size: 10px; }
  ::-webkit-scrollbar { width: 4px; } ::-webkit-scrollbar-track { background: #0f172a; } ::-webkit-scrollbar-thumb { background: #334155; border-radius: 2px; }
  table { border-collapse: collapse; width: 100%; }
  th { color: #64748b; font-size: 11px; text-transform: uppercase; letter-spacing: 1px; padding: 8px 12px; text-align: left; border-bottom: 1px solid #1e293b; }
  td { padding: 8px 12px; font-size: 12px; color: #cbd5e1; border-bottom: 1px solid #0f172a; }
  tr:hover td { background: #1e293b22; }
</style>
</head>
<body style="background:#020617;color:#f1f5f9;margin:0;padding:0;">

<!-- Header -->
<header style="background:#0a0f1e;border-bottom:1px solid #1e293b;padding:14px 24px;display:flex;align-items:center;justify-content:space-between;">
  <div style="display:flex;align-items:center;gap:16px;">
    <div style="color:#10b981;font-size:18px;font-weight:bold;letter-spacing:2px;">SIMPLEX</div>
    <div style="color:#334155;font-size:12px;" id="version">Dashboard</div>
    <div style="display:flex;align-items:center;gap:6px;">
      <span class="pulse" style="width:8px;height:8px;border-radius:50%;background:#10b981;display:inline-block;"></span>
      <span style="color:#10b981;font-size:11px;">LIVE</span>
    </div>
  </div>
  <div style="display:flex;align-items:center;gap:24px;">
    <div style="font-size:11px;color:#64748b;">Filler: <span style="color:#94a3b8;" id="filler-address">-</span></div>
    <div style="font-size:11px;color:#64748b;">Uptime: <span style="color:#94a3b8;" id="uptime">-</span></div>
    <div style="font-size:11px;color:#64748b;">Chains: <span style="color:#94a3b8;" id="chain-list">-</span></div>
  </div>
</header>

<main style="padding:20px;max-width:1600px;margin:0 auto;">

  <!-- Stats Row -->
  <div style="display:grid;grid-template-columns:repeat(7,1fr);gap:12px;margin-bottom:20px;">
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Orders Detected</div>
      <div style="font-size:28px;font-weight:bold;color:#38bdf8;" id="stat-detected">0</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Orders Filled</div>
      <div style="font-size:28px;font-weight:bold;color:#10b981;" id="stat-filled">0</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Bids Submitted</div>
      <div style="font-size:28px;font-weight:bold;color:#818cf8;" id="stat-bids">0</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Bid Success Rate</div>
      <div style="font-size:28px;font-weight:bold;color:#10b981;" id="stat-success-rate">-</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Failed Bids</div>
      <div style="font-size:28px;font-weight:bold;color:#ef4444;" id="stat-failed">0</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Pending Retraction</div>
      <div style="font-size:28px;font-weight:bold;color:#f59e0b;" id="stat-pending">0</div>
    </div>
    <div class="stat-card">
      <div style="color:#64748b;font-size:10px;letter-spacing:1px;text-transform:uppercase;margin-bottom:8px;">Est. Profit (USD)</div>
      <div style="font-size:28px;font-weight:bold;color:#94a3b8;" id="stat-profit">-</div>
      <div style="font-size:9px;color:#334155;margin-top:4px;" id="stat-profit-sub">last 24h, stables + exotic@ask</div>
    </div>
  </div>

  <!-- Charts + Balances Row -->
  <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-bottom:20px;">

    <!-- Price Curves -->
    <div class="panel" style="padding:20px;">
      <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">Price Curves</div>
      <div id="charts-container">
        <div style="color:#475569;font-size:12px;text-align:center;padding:40px 0;">Loading curves...</div>
      </div>
    </div>

    <!-- Account Balances -->
    <div class="panel" style="padding:20px;">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">
        <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;">Account Balances</div>
        <button onclick="refreshBalances()" style="color:#38bdf8;font-size:10px;background:none;border:1px solid #1e3a5f;border-radius:4px;padding:3px 8px;cursor:pointer;">Refresh</button>
      </div>
      <div id="balances-loading" style="color:#475569;font-size:12px;text-align:center;padding:40px 0;">Loading balances...</div>
      <table id="balances-table" style="display:none;">
        <thead><tr><th>Chain</th><th>USDC</th><th>USDT</th><th>Exotic</th><th>Native</th></tr></thead>
        <tbody id="balances-body"></tbody>
      </table>
    </div>
  </div>

  <!-- Hyperbridge Balance (shown only when connected) -->
  <div id="hb-panel" class="panel" style="display:none;padding:16px 20px;margin-bottom:20px;">
    <div style="display:flex;align-items:center;justify-content:space-between;">
      <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;">Hyperbridge Account</div>
      <div style="display:flex;align-items:center;gap:20px;">
        <span style="font-size:11px;color:#64748b;">Address: <span id="hb-address" style="color:#94a3b8;font-family:monospace;">-</span></span>
        <span style="font-size:11px;color:#64748b;">Free: <span id="hb-free" style="color:#10b981;font-size:14px;font-weight:bold;">-</span></span>
        <span style="font-size:11px;color:#64748b;">Reserved: <span id="hb-reserved" style="color:#f59e0b;">-</span></span>
        <span id="hb-symbol" style="font-size:10px;color:#475569;"></span>
      </div>
    </div>
  </div>

  <!-- Balance History Charts -->
  <div class="panel" style="padding:20px;margin-bottom:20px;">
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">
      <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;">Balance History</div>
      <div style="display:flex;gap:4px;" id="range-buttons"></div>
    </div>
    <div id="balance-history-container">
      <div style="color:#475569;font-size:12px;text-align:center;padding:20px 0;">Waiting for balance data...</div>
    </div>
  </div>

  <!-- Profit Charts -->
  <div class="panel" style="padding:20px;margin-bottom:20px;">
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">
      <div style="display:flex;align-items:baseline;gap:12px;">
        <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;">Estimated Profit</div>
        <div style="font-size:10px;color:#334155;" id="profit-range-label">delta vs. period start</div>
      </div>
    </div>
    <div id="profit-charts-container">
      <div style="color:#475569;font-size:12px;text-align:center;padding:20px 0;">Waiting for balance data...</div>
    </div>
  </div>

  <!-- Activity + Bid History -->
  <div style="display:grid;grid-template-columns:1fr 2fr;gap:16px;">

    <!-- Live Activity Feed -->
    <div class="panel" style="padding:20px;">
      <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">
        Live Activity
        <span class="pulse" style="display:inline-block;width:6px;height:6px;border-radius:50%;background:#10b981;margin-left:8px;vertical-align:middle;"></span>
      </div>
      <div id="activity-feed" style="height:380px;overflow-y:auto;">
        <div style="color:#475569;font-size:12px;text-align:center;padding:40px 0;">Waiting for events...</div>
      </div>
    </div>

    <!-- Bid History -->
    <div class="panel" style="padding:20px;">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;border-bottom:1px solid #1e293b;padding-bottom:10px;">
        <div style="color:#94a3b8;font-size:11px;letter-spacing:1px;text-transform:uppercase;">Bid History</div>
        <div style="display:flex;gap:8px;">
          <button onclick="loadBids(0)" style="color:#38bdf8;font-size:10px;background:none;border:1px solid #1e3a5f;border-radius:4px;padding:3px 8px;cursor:pointer;">Refresh</button>
          <button id="btn-retract" onclick="retractStaleBids()" style="color:#f59e0b;font-size:10px;background:none;border:1px solid #78350f;border-radius:4px;padding:3px 8px;cursor:pointer;" title="Retract all bids older than 1 hour">Retract Stale</button>
          <button id="btn-prev" onclick="changePage(-1)" style="color:#64748b;font-size:10px;background:none;border:1px solid #1e293b;border-radius:4px;padding:3px 8px;cursor:pointer;" disabled>← Prev</button>
          <span id="page-info" style="color:#475569;font-size:10px;line-height:24px;">-</span>
          <button id="btn-next" onclick="changePage(1)" style="color:#64748b;font-size:10px;background:none;border:1px solid #1e293b;border-radius:4px;padding:3px 8px;cursor:pointer;" disabled>Next →</button>
        </div>
      </div>
      <div style="height:380px;overflow-y:auto;">
        <table>
          <thead><tr><th>#</th><th>Commitment</th><th>Tx Hash</th><th>Status</th><th>Created</th><th>Retracted</th></tr></thead>
          <tbody id="bids-body"><tr><td colspan="6" style="text-align:center;color:#475569;padding:40px 0;">Loading...</td></tr></tbody>
        </table>
      </div>
    </div>
  </div>

</main>

<script>
const CONFIG = ${configJson};
const PAGE_SIZE = 25;
let currentPage = 0;
let totalBids = 0;
let charts = [];
const fxChartByIdx = {}; // strategy index -> Chart instance
const balHistoryCharts = {}; // 'stables' | symbol -> Chart instance
const profitCharts = {};     // 'stables' | symbol -> Chart instance
let balanceHistoryData = []; // full history for profit baseline
const MAX_BAL_POINTS = 200;
let sseConnected = false;
let activeRange = 86400000; // default 24h

const RANGES = [
  { ms: 3600000,    label: '1H'  },
  { ms: 21600000,   label: '6H'  },
  { ms: 86400000,   label: '24H' },
  { ms: 604800000,  label: '7D'  },
  { ms: 2592000000, label: '30D' },
  { ms: 0,          label: 'MAX' },
];

// ─── Utilities ────────────────────────────────────────────────────────────────

function fmt(addr) {
  if (!addr) return '-';
  return addr.slice(0, 8) + '...' + addr.slice(-6);
}

function fmtTime(iso) {
  const d = new Date(iso);
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString();
}

function fmtUptime(ms) {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return h + 'h ' + m + 'm ' + sec + 's';
  if (m > 0) return m + 'm ' + sec + 's';
  return sec + 's';
}

function fmtNum(n) {
  if (n === null || n === undefined) return '-';
  const f = parseFloat(n);
  if (isNaN(f)) return '-';
  if (f >= 1e6) return (f/1e6).toFixed(2) + 'M';
  if (f >= 1e3) return (f/1e3).toFixed(2) + 'k';
  return f.toFixed(2);
}

function chainName(id) {
  const names = {1:'Ethereum',56:'BSC',137:'Polygon',42161:'Arbitrum',8453:'Base',10:'Optimism',43114:'Avalanche',250:'Fantom',130:'Unichain',100:'Gnosis',97:'BSC Testnet',11155111:'Sepolia'};
  return names[id] || 'Chain ' + id;
}

// ─── SSE Connection ────────────────────────────────────────────────────────────

let currentEs = null;

function connectSSE() {
  if (currentEs) {
    currentEs.close();
    currentEs = null;
  }

  const es = new EventSource('/events');
  currentEs = es;

  es.addEventListener('connected', () => {
    sseConnected = true;
    console.log('SSE connected');
  });

  es.addEventListener('stats', (e) => {
    const data = JSON.parse(e.data);
    updateStats(data);
  });

  es.addEventListener('activity', (e) => {
    const data = JSON.parse(e.data);
    addActivity(data);
  });

  es.addEventListener('balances', (e) => {
    const data = JSON.parse(e.data);
    updateBalances(data);
  });

  es.addEventListener('hyperbridge_balance', (e) => {
    const data = JSON.parse(e.data);
    updateHyperbridgeBalance(data);
  });

  es.addEventListener('balance_history', () => {
    // Re-fetch with the current active range so charts respect the selected window
    fetchBalanceHistory(activeRange);
  });

  es.addEventListener('balance_point', (e) => {
    const data = JSON.parse(e.data);
    appendBalancePoint(data);
  });

  es.onerror = () => {
    if (currentEs !== es) return; // stale handler, ignore
    sseConnected = false;
    es.close();
    currentEs = null;
    setTimeout(connectSSE, 3000);
  };
}

// ─── Stats ─────────────────────────────────────────────────────────────────────

function updateStats(data) {
  document.getElementById('stat-detected').textContent = data.ordersDetected ?? 0;
  document.getElementById('stat-filled').textContent = data.ordersFilled ?? 0;
  document.getElementById('stat-bids').textContent = data.bidsTotal ?? 0;
  document.getElementById('stat-failed').textContent = data.bidsFailed ?? 0;
  document.getElementById('stat-pending').textContent = data.bidsPending ?? 0;
  document.getElementById('uptime').textContent = fmtUptime(data.uptimeMs ?? 0);

  totalBids = data.bidsTotal ?? 0;

  if (data.bidsTotal > 0) {
    const rate = ((data.bidsSuccess / data.bidsTotal) * 100).toFixed(1);
    document.getElementById('stat-success-rate').textContent = rate + '%';
  } else {
    document.getElementById('stat-success-rate').textContent = '-';
  }
}

// ─── Activity Feed ─────────────────────────────────────────────────────────────

function addActivity(event) {
  const feed = document.getElementById('activity-feed');

  // Remove placeholder
  const placeholder = feed.querySelector('[data-placeholder]');
  if (placeholder) placeholder.remove();

  const item = document.createElement('div');
  item.className = 'activity-item';
  item.style.cssText = 'padding:8px 4px;display:flex;align-items:flex-start;gap:8px;';

  const ts = new Date(event.timestamp);
  const timeStr = ts.toLocaleTimeString();

  let badge = '';
  let msg = '';

  switch(event.type) {
    case 'order_detected':
      badge = '<span class="tag-order">ORDER</span>';
      msg = 'New order <span class="hash">' + fmt(event.orderId) + '</span>';
      break;
    case 'order_filled':
      badge = '<span class="tag-fill">FILL</span>';
      msg = 'Filled <span class="hash">' + fmt(event.orderId) + '</span>' + (event.txHash ? ' tx: <span class="hash">' + fmt(event.txHash) + '</span>' : '');
      break;
    case 'order_executed':
      if (event.success) {
        badge = '<span class="tag-exec">EXEC ✓</span>';
        msg = 'Executed <span class="hash">' + fmt(event.orderId) + '</span> via <span style="color:#818cf8">' + (event.strategy||'?') + '</span>';
      } else {
        badge = '<span class="tag-skip">EXEC ✗</span>';
        msg = 'Failed <span class="hash">' + fmt(event.orderId) + '</span>' + (event.error ? ': ' + event.error.slice(0,60) : '');
      }
      break;
    case 'order_skipped':
      badge = '<span class="tag-skip">SKIP</span>';
      msg = 'Skipped <span class="hash">' + fmt(event.orderId) + '</span> — ' + (event.reason||'not profitable');
      break;
    case 'bid_submitted':
      badge = '<span class="tag-exec">BID</span>';
      msg = event.success ? '✓ Bid stored <span class="hash">' + fmt(event.commitment) + '</span>' : '✗ Bid failed: ' + (event.error||'').slice(0,60);
      break;
    default:
      badge = '<span class="tag-order">INFO</span>';
      msg = JSON.stringify(event).slice(0,80);
  }

  item.innerHTML = '<span style="color:#475569;font-size:10px;white-space:nowrap;margin-top:2px;">' + timeStr + '</span>' +
    badge + '<span style="font-size:11px;color:#94a3b8;flex:1;">' + msg + '</span>';

  feed.insertBefore(item, feed.firstChild);

  // Keep last 100 items
  while (feed.children.length > 100) {
    feed.removeChild(feed.lastChild);
  }
}

// ─── Balances ─────────────────────────────────────────────────────────────────

function updateBalances(balancesMap) {
  const tbody = document.getElementById('balances-body');
  const table = document.getElementById('balances-table');
  const loading = document.getElementById('balances-loading');

  tbody.innerHTML = '';
  const chains = Object.keys(balancesMap);
  if (chains.length === 0) {
    loading.style.display = 'block';
    table.style.display = 'none';
    return;
  }

  loading.style.display = 'none';
  table.style.display = 'table';

  let firstExoticSymbol = null;
  for (const chainId of chains) {
    const b = balancesMap[chainId];
    if (!firstExoticSymbol && b.exotics && b.exotics.length) firstExoticSymbol = b.exotics[0].symbol;
    const exoticHtml = (b.exotics && b.exotics.length)
      ? b.exotics.map(e => '<div>' + fmtNum(e.balance) + ' <span style="color:#a78bfa">' + e.symbol + '</span></div>').join('')
      : '<span style="color:#475569;">-</span>';
    const tr = document.createElement('tr');
    tr.innerHTML =
      '<td style="color:#94a3b8;">' + chainName(parseInt(chainId)) + ' <span style="color:#334155;font-size:10px;">(' + chainId + ')</span></td>' +
      '<td style="color:#10b981;">' + (b.usdc !== null ? fmtNum(b.usdc) + ' USDC' : '<span style="color:#475569;">N/A</span>') + '</td>' +
      '<td style="color:#10b981;">' + (b.usdt !== null ? fmtNum(b.usdt) + ' USDT' : '<span style="color:#475569;">N/A</span>') + '</td>' +
      '<td style="color:#a78bfa;">' + exoticHtml + '</td>' +
      '<td style="color:#94a3b8;">' + (b.native !== null ? parseFloat(b.native).toFixed(4) + ' ' + (b.nativeSymbol||'ETH') : '<span style="color:#475569;">N/A</span>') + '</td>';
    tbody.appendChild(tr);
  }

  // Update FX chart y-axis labels with the actual exotic token symbol
  if (firstExoticSymbol) {
    for (const idx of Object.keys(fxChartByIdx)) {
      const chart = fxChartByIdx[idx];
      const yTitle = chart.options.scales.y.title;
      if (yTitle.text !== firstExoticSymbol + ' per USD') {
        yTitle.text = firstExoticSymbol + ' per USD';
        chart.data.datasets[0].label = 'Bid (buy ' + firstExoticSymbol + ')';
        chart.data.datasets[1].label = 'Ask (sell ' + firstExoticSymbol + ')';
        chart.update();
      }
    }
  }
}

async function refreshBalances() {
  try {
    const res = await fetch('/api/balances');
    if (res.ok) updateBalances(await res.json());
  } catch(e) { console.error('Balance refresh failed:', e); }
}

// ─── Bid History ──────────────────────────────────────────────────────────────

async function loadBids(offset) {
  try {
    const res = await fetch('/api/bids?limit=' + PAGE_SIZE + '&offset=' + offset);
    if (!res.ok) return;
    const data = await res.json();
    renderBids(data.bids, data.total, offset);
  } catch(e) { console.error('Bid load failed:', e); }
}

function renderBids(bids, total, offset) {
  const tbody = document.getElementById('bids-body');
  if (!bids || bids.length === 0) {
    tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;color:#475569;padding:40px 0;">No bids yet</td></tr>';
    document.getElementById('page-info').textContent = '0 bids';
    document.getElementById('btn-prev').disabled = true;
    document.getElementById('btn-next').disabled = true;
    return;
  }

  tbody.innerHTML = '';
  bids.forEach(bid => {
    const tr = document.createElement('tr');
    let statusBadge;
    if (!bid.success) statusBadge = '<span class="badge-fail">Failed</span>';
    else if (bid.retracted) statusBadge = '<span class="badge-retracted">Retracted</span>';
    else statusBadge = '<span class="badge-success">Active</span>';

    tr.innerHTML =
      '<td style="color:#475569;">' + bid.id + '</td>' +
      '<td><span class="hash" title="' + (bid.commitment||'') + '">' + fmt(bid.commitment) + '</span></td>' +
      '<td><span class="hash" title="' + (bid.extrinsicHash||'') + '">' + (bid.extrinsicHash ? fmt(bid.extrinsicHash) : '-') + '</span></td>' +
      '<td>' + statusBadge + '</td>' +
      '<td style="color:#64748b;font-size:11px;">' + fmtTime(bid.createdAt) + '</td>' +
      '<td style="color:#64748b;font-size:11px;">' + (bid.retractedAt ? fmtTime(bid.retractedAt) : '-') + '</td>';
    tbody.appendChild(tr);
  });

  const page = Math.floor(offset / PAGE_SIZE) + 1;
  const totalPages = Math.ceil(total / PAGE_SIZE);
  document.getElementById('page-info').textContent = 'p.' + page + '/' + totalPages + ' (' + total + ')';
  document.getElementById('btn-prev').disabled = offset === 0;
  document.getElementById('btn-next').disabled = offset + PAGE_SIZE >= total;
  currentPage = Math.floor(offset / PAGE_SIZE);
}

function changePage(dir) {
  const offset = (currentPage + dir) * PAGE_SIZE;
  if (offset < 0) return;
  loadBids(offset);
}

// ─── Charts ───────────────────────────────────────────────────────────────────

function renderCharts() {
  const container = document.getElementById('charts-container');
  container.innerHTML = '';

  if (!CONFIG.strategies || CONFIG.strategies.length === 0) {
    container.innerHTML = '<div style="color:#475569;font-size:12px;text-align:center;padding:40px 0;">No strategy curves configured</div>';
    return;
  }

  CONFIG.strategies.forEach((strategy, idx) => {
    if (strategy.type === 'basic') {
      const wrapper = document.createElement('div');
      wrapper.style.cssText = 'margin-bottom:16px;';
      wrapper.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">BASIC — BPS SPREAD CURVE</div><canvas id="chart-' + idx + '" height="120"></canvas>';
      container.appendChild(wrapper);

      const pts = strategy.bpsCurve.map(p => ({ x: parseFloat(p.amount), y: p.value })).sort((a,b) => a.x - b.x);
      new Chart(document.getElementById('chart-' + idx), {
        type: 'line',
        data: {
          datasets: [{
            label: 'BPS',
            data: pts,
            borderColor: '#10b981',
            backgroundColor: 'rgba(16,185,129,0.08)',
            fill: true,
            tension: 0.3,
            pointRadius: 4,
            pointBackgroundColor: '#10b981',
          }]
        },
        options: {
          responsive: true,
          scales: {
            x: { type: 'linear', title: { display: true, text: 'Order Value (USD)', color: '#475569', font: { size: 10 } }, ticks: { color: '#475569', font: { size: 10 } }, grid: { color: '#1e293b' } },
            y: { title: { display: true, text: 'BPS', color: '#475569', font: { size: 10 } }, ticks: { color: '#475569', font: { size: 10 } }, grid: { color: '#1e293b' } }
          },
          plugins: { legend: { display: false } },
          animation: false,
        }
      });

    } else if (strategy.type === 'hyperfx') {
      const wrapper = document.createElement('div');
      wrapper.style.cssText = 'margin-bottom:16px;';
      wrapper.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">HYPERFX — BID / ASK PRICE CURVES (max: $' + strategy.maxOrderUsd + ')</div><canvas id="chart-' + idx + '" height="140"></canvas>';
      container.appendChild(wrapper);

      const bidPts = strategy.bidPriceCurve.map(p => ({ x: parseFloat(p.amount), y: parseFloat(p.price) })).sort((a,b) => a.x - b.x);
      const askPts = strategy.askPriceCurve.map(p => ({ x: parseFloat(p.amount), y: parseFloat(p.price) })).sort((a,b) => a.x - b.x);

      fxChartByIdx[idx] = new Chart(document.getElementById('chart-' + idx), {
        type: 'line',
        data: {
          datasets: [
            {
              label: 'Bid (buy exotic)',
              data: bidPts,
              borderColor: '#3b82f6',
              backgroundColor: 'rgba(59,130,246,0.06)',
              fill: false,
              tension: 0.3,
              pointRadius: 4,
              pointBackgroundColor: '#3b82f6',
            },
            {
              label: 'Ask (sell exotic)',
              data: askPts,
              borderColor: '#f59e0b',
              backgroundColor: 'rgba(245,158,11,0.06)',
              fill: false,
              tension: 0.3,
              pointRadius: 4,
              pointBackgroundColor: '#f59e0b',
            }
          ]
        },
        options: {
          responsive: true,
          scales: {
            x: { type: 'linear', title: { display: true, text: 'Order Value (USD)', color: '#475569', font: { size: 10 } }, ticks: { color: '#475569', font: { size: 10 } }, grid: { color: '#1e293b' } },
            y: { title: { display: true, text: '? per USD', color: '#475569', font: { size: 10 } }, ticks: { color: '#475569', font: { size: 10 } }, grid: { color: '#1e293b' } }
          },
          plugins: { legend: { display: true, labels: { color: '#94a3b8', font: { size: 10 } } } },
          animation: false,
        }
      });
    }
  });
}

// ─── Hyperbridge Balance ──────────────────────────────────────────────────────

function updateHyperbridgeBalance(data) {
  if (!data) return;
  document.getElementById('hb-panel').style.display = 'block';
  document.getElementById('hb-address').textContent = data.address ? fmt(data.address) : '-';
  document.getElementById('hb-address').title = data.address || '';
  const dec = data.decimals || 12;
  const freeNum = parseFloat(data.free || '0') / Math.pow(10, dec);
  const resNum = parseFloat(data.reserved || '0') / Math.pow(10, dec);
  document.getElementById('hb-free').textContent = freeNum.toFixed(4) + ' ' + (data.symbol || 'BRIDGE');
  document.getElementById('hb-reserved').textContent = resNum.toFixed(4) + ' reserved';
  document.getElementById('hb-symbol').textContent = '';
}

// ─── Range Selector ───────────────────────────────────────────────────────────

function buildRangeButtons() {
  const container = document.getElementById('range-buttons');
  if (!container) return;
  container.innerHTML = '';
  for (const r of RANGES) {
    const btn = document.createElement('button');
    btn.id = 'rbtn-' + r.label;
    btn.textContent = r.label;
    btn.onclick = () => selectRange(r.ms);
    btn.style.cssText = 'font-size:10px;background:none;border:1px solid #1e293b;border-radius:4px;padding:2px 7px;cursor:pointer;font-family:inherit;';
    container.appendChild(btn);
  }
  updateRangeButtonStyles();
}

function updateRangeButtonStyles() {
  for (const r of RANGES) {
    const btn = document.getElementById('rbtn-' + r.label);
    if (!btn) continue;
    const active = r.ms === activeRange;
    btn.style.color = active ? '#10b981' : '#64748b';
    btn.style.borderColor = active ? '#065f46' : '#1e293b';
  }
}

function fmtRangeLabel(ms) {
  if (ms === 0)          return 'all time';
  if (ms === 3600000)    return 'last 1h';
  if (ms === 21600000)   return 'last 6h';
  if (ms === 86400000)   return 'last 24h';
  if (ms === 604800000)  return 'last 7d';
  if (ms === 2592000000) return 'last 30d';
  return 'selected range';
}

async function selectRange(rangeMs) {
  activeRange = rangeMs;
  updateRangeButtonStyles();
  const label = document.getElementById('profit-range-label');
  if (label) label.textContent = 'delta vs. ' + fmtRangeLabel(rangeMs) + ' start';
  document.getElementById('stat-profit-sub').textContent = fmtRangeLabel(rangeMs) + ', stables + exotic@ask';
  await fetchBalanceHistory(rangeMs);
}

async function fetchBalanceHistory(rangeMs) {
  try {
    const since = rangeMs > 0 ? Date.now() - rangeMs : 0;
    const url = '/api/balance-history' + (since > 0 ? '?since=' + since : '');
    const res = await fetch(url);
    if (res.ok) {
      const data = await res.json();
      if (data.length > 0) initBalanceHistoryCharts(data);
    }
  } catch(e) {}
}

// ─── Balance History Charts ───────────────────────────────────────────────────

function fmtAxisTime(ts) {
  const d = new Date(ts);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function makeBalChartOpts(titleText) {
  return {
    responsive: true,
    animation: false,
    scales: {
      x: {
        type: 'linear',
        ticks: {
          color: '#475569',
          font: { size: 9 },
          maxTicksLimit: 8,
          callback: function(v) { return fmtAxisTime(v); },
        },
        grid: { color: '#1e293b' },
      },
      y: {
        title: { display: true, text: titleText, color: '#475569', font: { size: 10 } },
        ticks: { color: '#475569', font: { size: 10 } },
        grid: { color: '#1e293b' },
        beginAtZero: false,
      }
    },
    plugins: {
      legend: { display: true, labels: { color: '#94a3b8', font: { size: 10 } } },
      tooltip: {
        callbacks: {
          title: function(items) { return fmtAxisTime(items[0].parsed.x); }
        }
      }
    },
  };
}

function buildBalHistoryGrid(exoticSymbols, hasStables) {
  const container = document.getElementById('balance-history-container');
  container.innerHTML = '';
  const chartCount = (hasStables ? 1 : 0) + exoticSymbols.length;
  if (chartCount === 0) {
    container.innerHTML = '<div style="color:#475569;font-size:12px;text-align:center;padding:20px 0;">No balance data yet</div>';
    return false;
  }
  const cols = chartCount > 1 ? 'repeat(2,1fr)' : '1fr';
  const grid = document.createElement('div');
  grid.id = 'bh-grid';
  grid.style.cssText = 'display:grid;grid-template-columns:' + cols + ';gap:16px;';
  container.appendChild(grid);

  if (hasStables) {
    const w = document.createElement('div');
    w.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">STABLECOIN BALANCES (USD)</div><canvas id="bhc-stables" height="130"></canvas>';
    grid.appendChild(w);
  }
  for (const sym of exoticSymbols) {
    const safeId = 'bhc-' + sym.replace(/[^a-z0-9]/gi, '_');
    const w = document.createElement('div');
    w.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">' + sym + ' BALANCE</div><canvas id="' + safeId + '" height="130"></canvas>';
    grid.appendChild(w);
  }
  return true;
}

function initBalanceHistoryCharts(history) {
  if (!history || history.length === 0) return;

  // Always update profit charts / card (they use the full history as baseline)
  renderProfitCharts(history);

  const exoticSymbols = [...new Set(history.flatMap(pt => Object.keys(pt.exotics || {})))];
  const hasStables = history.some(pt => pt.usdc > 0 || pt.usdt > 0);

  // If charts already exist just replace data
  if (Object.keys(balHistoryCharts).length > 0) {
    const labels = history.map(pt => pt.timestamp);
    if (balHistoryCharts['stables']) {
      const c = balHistoryCharts['stables'];
      c.data.labels = labels;
      c.data.datasets[0].data = history.map(pt => pt.usdc);
      c.data.datasets[1].data = history.map(pt => pt.usdt);
      c.update();
    }
    for (const sym of exoticSymbols) {
      if (balHistoryCharts[sym]) {
        const c = balHistoryCharts[sym];
        c.data.labels = labels;
        c.data.datasets[0].data = history.map(pt => pt.exotics[sym] || 0);
        c.update();
      }
    }
    return;
  }

  if (!buildBalHistoryGrid(exoticSymbols, hasStables)) return;

  const labels = history.map(pt => pt.timestamp);

  if (hasStables) {
    balHistoryCharts['stables'] = new Chart(document.getElementById('bhc-stables'), {
      type: 'line',
      data: {
        labels,
        datasets: [
          { label: 'USDC', data: history.map(pt => pt.usdc), borderColor: '#10b981', backgroundColor: 'rgba(16,185,129,0.06)', fill: false, tension: 0.3, pointRadius: 2 },
          { label: 'USDT', data: history.map(pt => pt.usdt), borderColor: '#06b6d4', backgroundColor: 'rgba(6,182,212,0.06)', fill: false, tension: 0.3, pointRadius: 2 },
        ]
      },
      options: makeBalChartOpts('USD'),
    });
  }

  for (const sym of exoticSymbols) {
    const safeId = 'bhc-' + sym.replace(/[^a-z0-9]/gi, '_');
    balHistoryCharts[sym] = new Chart(document.getElementById(safeId), {
      type: 'line',
      data: {
        labels,
        datasets: [{ label: sym, data: history.map(pt => pt.exotics[sym] || 0), borderColor: '#a78bfa', backgroundColor: 'rgba(167,139,250,0.06)', fill: false, tension: 0.3, pointRadius: 2 }]
      },
      options: makeBalChartOpts(sym),
    });
  }
}

function appendBalancePoint(point) {
  if (Object.keys(balHistoryCharts).length === 0) {
    initBalanceHistoryCharts([point]);
    return;
  }

  // Keep local history in sync for profit baseline
  balanceHistoryData.push(point);
  if (balanceHistoryData.length > MAX_BAL_POINTS) balanceHistoryData.shift();

  appendProfitPoint(point);

  if (balHistoryCharts['stables']) {
    const c = balHistoryCharts['stables'];
    c.data.labels.push(point.timestamp);
    c.data.datasets[0].data.push(point.usdc);
    c.data.datasets[1].data.push(point.usdt);
    while (c.data.labels.length > MAX_BAL_POINTS) { c.data.labels.shift(); c.data.datasets.forEach(ds => ds.data.shift()); }
    c.update();
  }

  for (const [sym, val] of Object.entries(point.exotics || {})) {
    if (balHistoryCharts[sym]) {
      const c = balHistoryCharts[sym];
      c.data.labels.push(point.timestamp);
      c.data.datasets[0].data.push(val);
      while (c.data.labels.length > MAX_BAL_POINTS) { c.data.labels.shift(); c.data.datasets[0].data.shift(); }
      c.update();
    }
  }
}

// ─── Profit Charts ────────────────────────────────────────────────────────────

function computeProfitSeries(history) {
  if (!history || history.length === 0) return [];
  const base = history[0];
  return history.map(pt => ({
    timestamp: pt.timestamp,
    usdc: pt.usdc - base.usdc,
    usdt: pt.usdt - base.usdt,
    exotics: Object.fromEntries(
      Object.entries(pt.exotics || {}).map(([k, v]) => [k, v - (base.exotics[k] || 0)])
    ),
  }));
}

// Returns the ask price (exotic per USD) from the hyperfx strategy at a
// reference order size — used to convert exotic profit into USD.
// The ask price is "how many exotic tokens the filler delivers per $1",
// so USD_value = exotic_amount / ask_price.
function getExoticAskPricePerUsd() {
  for (const s of CONFIG.strategies) {
    if (s.type === 'hyperfx' && s.askPriceCurve && s.askPriceCurve.length > 0) {
      // Use the midpoint of the curve as a representative rate
      const mid = s.askPriceCurve[Math.floor(s.askPriceCurve.length / 2)];
      const rate = parseFloat(mid.price);
      if (rate > 0) return rate;
    }
  }
  return null;
}

function updateProfitCard() {
  if (balanceHistoryData.length < 2) return;
  const last = computeProfitSeries(balanceHistoryData).at(-1);
  let total = (last.usdc || 0) + (last.usdt || 0);

  // Convert exotic profit to USD using the strategy's ask price
  const askRate = getExoticAskPricePerUsd();
  if (askRate) {
    for (const delta of Object.values(last.exotics || {})) {
      total += delta / askRate;
    }
  }

  const el = document.getElementById('stat-profit');
  if (!el) return;
  el.textContent = (total >= 0 ? '+' : '') + fmtNum(total);
  el.style.color = total > 0 ? '#10b981' : total < 0 ? '#ef4444' : '#94a3b8';
}

function renderProfitCharts(history) {
  balanceHistoryData = history;
  updateProfitCard();
  if (history.length < 2) return;

  const profitSeries = computeProfitSeries(history);
  const exoticSymbols = [...new Set(history.flatMap(pt => Object.keys(pt.exotics || {})))];
  const hasStables = history.some(pt => pt.usdc !== 0 || pt.usdt !== 0);

  // If charts exist, just update data
  if (Object.keys(profitCharts).length > 0) {
    const labels = profitSeries.map(pt => pt.timestamp);
    if (profitCharts['stables']) {
      const c = profitCharts['stables'];
      c.data.labels = labels;
      c.data.datasets[0].data = profitSeries.map(pt => pt.usdc);
      c.data.datasets[1].data = profitSeries.map(pt => pt.usdt);
      c.update();
    }
    for (const sym of exoticSymbols) {
      if (profitCharts[sym]) {
        const c = profitCharts[sym];
        c.data.labels = labels;
        c.data.datasets[0].data = profitSeries.map(pt => pt.exotics[sym] || 0);
        c.update();
      }
    }
    return;
  }

  const container = document.getElementById('profit-charts-container');
  container.innerHTML = '';
  const chartCount = (hasStables ? 1 : 0) + exoticSymbols.length;
  if (chartCount === 0) return;

  const grid = document.createElement('div');
  grid.style.cssText = 'display:grid;grid-template-columns:' + (chartCount > 1 ? 'repeat(2,1fr)' : '1fr') + ';gap:16px;';
  container.appendChild(grid);

  const labels = profitSeries.map(pt => pt.timestamp);

  if (hasStables) {
    const w = document.createElement('div');
    w.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">STABLECOIN PROFIT (Δ USD)</div><canvas id="pc-stables" height="130"></canvas>';
    grid.appendChild(w);
    profitCharts['stables'] = new Chart(document.getElementById('pc-stables'), {
      type: 'line',
      data: {
        labels,
        datasets: [
          { label: 'USDC Δ', data: profitSeries.map(pt => pt.usdc), borderColor: '#10b981', backgroundColor: 'rgba(16,185,129,0.06)', fill: false, tension: 0.3, pointRadius: 2 },
          { label: 'USDT Δ', data: profitSeries.map(pt => pt.usdt), borderColor: '#06b6d4', backgroundColor: 'rgba(6,182,212,0.06)', fill: false, tension: 0.3, pointRadius: 2 },
        ]
      },
      options: makeProfitChartOpts('Δ USD'),
    });
  }

  for (const sym of exoticSymbols) {
    const safeId = 'pc-' + sym.replace(/[^a-z0-9]/gi, '_');
    const w = document.createElement('div');
    w.innerHTML = '<div style="color:#64748b;font-size:10px;margin-bottom:8px;letter-spacing:1px;">' + sym + ' PROFIT (Δ)</div><canvas id="' + safeId + '" height="130"></canvas>';
    grid.appendChild(w);
    profitCharts[sym] = new Chart(document.getElementById(safeId), {
      type: 'line',
      data: {
        labels,
        datasets: [{ label: sym + ' Δ', data: profitSeries.map(pt => pt.exotics[sym] || 0), borderColor: '#a78bfa', backgroundColor: 'rgba(167,139,250,0.06)', fill: false, tension: 0.3, pointRadius: 2 }]
      },
      options: makeProfitChartOpts('Δ ' + sym),
    });
  }
}

function makeProfitChartOpts(titleText) {
  const opts = makeBalChartOpts(titleText);
  // Add a zero-baseline annotation via borderDash on the grid line at y=0
  opts.scales.y.grid = {
    color: function(ctx) {
      return ctx.tick.value === 0 ? '#475569' : '#1e293b';
    },
    lineWidth: function(ctx) {
      return ctx.tick.value === 0 ? 1.5 : 1;
    },
  };
  return opts;
}

function appendProfitPoint(point) {
  if (balanceHistoryData.length === 0) return;
  const base = balanceHistoryData[0];
  const profitPoint = {
    timestamp: point.timestamp,
    usdc: point.usdc - base.usdc,
    usdt: point.usdt - base.usdt,
    exotics: Object.fromEntries(
      Object.entries(point.exotics || {}).map(([k, v]) => [k, v - (base.exotics[k] || 0)])
    ),
  };

  if (profitCharts['stables']) {
    const c = profitCharts['stables'];
    c.data.labels.push(profitPoint.timestamp);
    c.data.datasets[0].data.push(profitPoint.usdc);
    c.data.datasets[1].data.push(profitPoint.usdt);
    while (c.data.labels.length > MAX_BAL_POINTS) { c.data.labels.shift(); c.data.datasets.forEach(ds => ds.data.shift()); }
    c.update();
  }
  for (const [sym, val] of Object.entries(profitPoint.exotics)) {
    if (profitCharts[sym]) {
      const c = profitCharts[sym];
      c.data.labels.push(profitPoint.timestamp);
      c.data.datasets[0].data.push(val);
      while (c.data.labels.length > MAX_BAL_POINTS) { c.data.labels.shift(); c.data.datasets[0].data.shift(); }
      c.update();
    }
  }
  updateProfitCard();
}

// ─── Retract Stale Bids ───────────────────────────────────────────────────────

async function retractStaleBids() {
  const btn = document.getElementById('btn-retract');
  btn.disabled = true;
  btn.textContent = 'Retracting...';
  try {
    const res = await fetch('/api/retract-stale', { method: 'POST' });
    const data = await res.json();
    if (res.ok) {
      btn.textContent = 'Queued ' + data.queued;
      setTimeout(() => { btn.textContent = 'Retract Stale'; btn.disabled = false; loadBids(0); }, 3000);
    } else {
      btn.textContent = data.error || 'Error';
      setTimeout(() => { btn.textContent = 'Retract Stale'; btn.disabled = false; }, 3000);
    }
  } catch(e) {
    btn.textContent = 'Error';
    setTimeout(() => { btn.textContent = 'Retract Stale'; btn.disabled = false; }, 3000);
  }
}

// ─── Init ─────────────────────────────────────────────────────────────────────

async function init() {
  // Render static info
  document.getElementById('filler-address').textContent = fmt(CONFIG.fillerAddress);
  document.getElementById('chain-list').textContent = CONFIG.chains.map(chainName).join(', ');

  // Render charts from config
  renderCharts();

  // Load initial stats
  try {
    const res = await fetch('/api/stats');
    if (res.ok) updateStats(await res.json());
  } catch(e) {}

  // Load initial bids
  loadBids(0);

  // Load initial balances
  refreshBalances();

  // Build range selector and load initial balance history (default 24h)
  buildRangeButtons();
  await fetchBalanceHistory(activeRange);

  // Connect SSE
  connectSSE();

  // Set placeholder with data-placeholder attribute
  const feed = document.getElementById('activity-feed');
  const ph = feed.querySelector('div');
  if (ph) ph.setAttribute('data-placeholder', '1');

  // Uptime ticker (update from server stats every 5s)
  setInterval(async () => {
    try {
      const res = await fetch('/api/stats');
      if (res.ok) {
        const data = await res.json();
        document.getElementById('uptime').textContent = fmtUptime(data.uptimeMs);
        updateStats(data);
      }
    } catch(e) {}
  }, 5000);

  // Auto-refresh bids every 30s
  setInterval(() => loadBids(currentPage * PAGE_SIZE), 30000);
}

document.addEventListener('DOMContentLoaded', init);
</script>
</body>
</html>`;
}
