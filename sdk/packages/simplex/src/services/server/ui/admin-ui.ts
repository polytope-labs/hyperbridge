/**
 * Single-file admin UI served at `/` by the AdminServer. Kept as a TS string
 * (rather than an .html asset) so it bundles identically under tsup, tsx and
 * vitest with no loader configuration. The inline script deliberately avoids
 * template literals so this file stays a plain literal.
 */
export const ADMIN_UI_HTML = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Simplex — Price Curves</title>
<style>
	:root {
		--bg: #f6f7f9; --card: #ffffff; --text: #1a1d21; --muted: #6b7280;
		--border: #e2e5ea; --accent: #2563eb; --accent-text: #ffffff;
		--danger: #b91c1c; --ok: #15803d; --input-bg: #ffffff;
	}
	@media (prefers-color-scheme: dark) {
		:root {
			--bg: #101318; --card: #181c23; --text: #e5e7eb; --muted: #9ca3af;
			--border: #2a303b; --accent: #3b82f6; --accent-text: #ffffff;
			--danger: #f87171; --ok: #4ade80; --input-bg: #10131a;
		}
	}
	* { box-sizing: border-box; }
	body {
		margin: 0; padding: 24px; background: var(--bg); color: var(--text);
		font: 14px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
	}
	.wrap { max-width: 860px; margin: 0 auto; }
	h1 { font-size: 20px; margin: 0 0 4px; }
	.subtitle { color: var(--muted); margin: 0 0 20px; }
	.card {
		background: var(--card); border: 1px solid var(--border); border-radius: 10px;
		padding: 18px 20px; margin-bottom: 18px;
	}
	.card h2 { font-size: 15px; margin: 0 0 2px; }
	.badge {
		display: inline-block; font-size: 11px; padding: 1px 8px; border-radius: 999px;
		border: 1px solid var(--border); color: var(--muted); margin-left: 8px; vertical-align: 1px;
	}
	.side { margin-top: 14px; }
	.side h3 { font-size: 13px; margin: 0 0 8px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
	table { border-collapse: collapse; }
	th { font-size: 11px; text-align: left; color: var(--muted); font-weight: 500; padding: 0 6px 4px 2px; }
	td { padding: 2px 6px 2px 2px; }
	input {
		width: 130px; padding: 5px 8px; border: 1px solid var(--border); border-radius: 6px;
		background: var(--input-bg); color: var(--text); font: inherit;
	}
	input:focus { outline: none; border-color: var(--accent); }
	button {
		padding: 6px 14px; border: 1px solid var(--border); border-radius: 6px;
		background: var(--card); color: var(--text); font: inherit; cursor: pointer;
	}
	button:hover { border-color: var(--accent); }
	button.primary { background: var(--accent); border-color: var(--accent); color: var(--accent-text); }
	button.icon { padding: 4px 9px; color: var(--muted); }
	button:disabled { opacity: 0.5; cursor: default; }
	.row { display: flex; gap: 24px; flex-wrap: wrap; align-items: flex-start; }
	.actions { margin-top: 14px; display: flex; gap: 10px; align-items: center; }
	.msg { font-size: 13px; }
	.msg.error { color: var(--danger); }
	.msg.ok { color: var(--ok); }
	svg { display: block; margin-top: 6px; }
	.curve-line { stroke: var(--accent); stroke-width: 2; fill: none; }
	.curve-dot { fill: var(--accent); }
	.axis { stroke: var(--border); stroke-width: 1; }
	.muted { color: var(--muted); }
</style>
</head>
<body>
<div class="wrap">
	<h1>Simplex — Price Curves</h1>
	<p class="subtitle">Changes apply in memory immediately and are lost on restart; the TOML config is re-read on boot.
	Strategy numbers are positions in the config's <code>[[strategies]]</code> list (0-based); only FX strategies are shown.</p>
	<div id="app" class="muted">Loading…</div>
</div>
<script>
(function () {
	'use strict'

	var app = document.getElementById('app')

	function el(tag, attrs, children) {
		var node = document.createElement(tag)
		Object.keys(attrs || {}).forEach(function (k) {
			if (k === 'class') node.className = attrs[k]
			else if (k === 'text') node.textContent = attrs[k]
			else if (k.indexOf('on') === 0) node.addEventListener(k.slice(2), attrs[k])
			else node.setAttribute(k, attrs[k])
		})
		;(children || []).forEach(function (c) { node.appendChild(c) })
		return node
	}

	function svgEl(tag, attrs) {
		var node = document.createElementNS('http://www.w3.org/2000/svg', tag)
		Object.keys(attrs || {}).forEach(function (k) { node.setAttribute(k, attrs[k]) })
		return node
	}

	// Renders a piecewise-linear preview of the points currently in the table.
	function renderPreview(container, points) {
		container.innerHTML = ''
		var parsed = points
			.map(function (p) { return { amount: parseFloat(p.amount), price: parseFloat(p.price) } })
			.filter(function (p) { return isFinite(p.amount) && isFinite(p.price) })
			.sort(function (a, b) { return a.amount - b.amount })
		if (parsed.length === 0) return

		var W = 320, H = 96, PAD = 10
		var svg = svgEl('svg', { width: W, height: H })
		var minA = parsed[0].amount, maxA = parsed[parsed.length - 1].amount
		var prices = parsed.map(function (p) { return p.price })
		var minP = Math.min.apply(null, prices), maxP = Math.max.apply(null, prices)
		var spanA = maxA - minA || 1, spanP = maxP - minP || 1
		function x(a) { return PAD + ((a - minA) / spanA) * (W - 2 * PAD) }
		function y(p) { return H - PAD - ((p - minP) / spanP) * (H - 2 * PAD) }

		svg.appendChild(svgEl('line', { x1: 0, y1: H - 1, x2: W, y2: H - 1, class: 'axis' }))
		var d = 'M 0 ' + y(parsed[0].price)
		parsed.forEach(function (p) { d += ' L ' + x(p.amount) + ' ' + y(p.price) })
		d += ' L ' + W + ' ' + y(parsed[parsed.length - 1].price)
		svg.appendChild(svgEl('path', { d: d, class: 'curve-line' }))
		parsed.forEach(function (p) {
			svg.appendChild(svgEl('circle', { cx: x(p.amount), cy: y(p.price), r: 3, class: 'curve-dot' }))
		})
		container.appendChild(svg)
	}

	// One editable side (bid or ask). Returns { root, getPoints }.
	function curveEditor(label, points) {
		var rows = points.map(function (p) { return { amount: p.amount, price: p.price } })
		var tbody = el('tbody')
		var preview = el('div')

		function currentPoints() { return rows.slice() }
		function refresh() {
			tbody.innerHTML = ''
			rows.forEach(function (row, i) {
				var amount = el('input', { value: row.amount, oninput: function (e) { row.amount = e.target.value; renderPreview(preview, currentPoints()) } })
				var price = el('input', { value: row.price, oninput: function (e) { row.price = e.target.value; renderPreview(preview, currentPoints()) } })
				var remove = el('button', { class: 'icon', text: '✕', title: 'Remove point', onclick: function () { rows.splice(i, 1); refresh() } })
				if (rows.length === 1) remove.disabled = true
				tbody.appendChild(el('tr', {}, [el('td', {}, [amount]), el('td', {}, [price]), el('td', {}, [remove])]))
			})
			renderPreview(preview, currentPoints())
		}

		var add = el('button', { text: '+ Add point', onclick: function () {
			var last = rows[rows.length - 1]
			rows.push({ amount: last ? last.amount : '0', price: last ? last.price : '1' })
			refresh()
		} })

		var root = el('div', { class: 'side' }, [
			el('h3', { text: label }),
			el('div', { class: 'row' }, [
				el('div', {}, [
					el('table', {}, [
						el('thead', {}, [el('tr', {}, [el('th', { text: 'Amount (USD)' }), el('th', { text: 'Price (per USD)' }), el('th')])]),
						tbody,
					]),
					el('div', { style: 'margin-top:6px' }, [add]),
				]),
				preview,
			]),
		])
		refresh()
		return { root: root, getPoints: currentPoints }
	}

	function strategyCard(strategy) {
		var title = el('h2', { text: 'Strategy #' + strategy.index + (strategy.exotic ? ' — ' + strategy.exotic : '') })

		if (strategy.pricingMode === 'venue') {
			title.appendChild(el('span', { class: 'badge', text: 'venue-priced' }))
			return el('div', { class: 'card' }, [
				title,
				el('p', { class: 'muted', text: 'Prices are derived from on-chain venues (Uniswap V4) and cannot be edited here.' }),
			])
		}

		var editors = {}
		var children = [title]
		if (strategy.bid) { editors.bid = curveEditor('Bid — filler buys exotic', strategy.bid); children.push(editors.bid.root) }
		if (strategy.ask) { editors.ask = curveEditor('Ask — filler sells exotic', strategy.ask); children.push(editors.ask.root) }

		var msg = el('span', { class: 'msg' })
		var apply = el('button', { class: 'primary', text: 'Apply', onclick: function () {
			var body = {}
			if (editors.bid) body.bidPriceCurve = editors.bid.getPoints()
			if (editors.ask) body.askPriceCurve = editors.ask.getPoints()
			apply.disabled = true
			msg.className = 'msg'
			msg.textContent = 'Applying…'
			fetch('/api/strategies/' + strategy.index + '/curves', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(body),
			}).then(function (res) {
				return res.json().then(function (data) {
					if (!res.ok) throw new Error(data.error || ('HTTP ' + res.status))
					msg.className = 'msg ok'
					msg.textContent = 'Applied ✓ (in memory only)'
				})
			}).catch(function (err) {
				msg.className = 'msg error'
				msg.textContent = err.message
			}).then(function () {
				apply.disabled = false
			})
		} })

		children.push(el('div', { class: 'actions' }, [apply, msg]))
		return el('div', { class: 'card' }, children)
	}

	fetch('/api/strategies').then(function (res) {
		if (!res.ok) throw new Error('HTTP ' + res.status)
		return res.json()
	}).then(function (data) {
		app.className = ''
		app.innerHTML = ''
		if (data.strategies.length === 0) {
			app.appendChild(el('p', { class: 'muted', text: 'No FX strategies configured.' }))
			return
		}
		data.strategies.forEach(function (s) { app.appendChild(strategyCard(s)) })
	}).catch(function (err) {
		app.textContent = 'Failed to load strategies: ' + err.message
	})
})()
</script>
</body>
</html>
`
