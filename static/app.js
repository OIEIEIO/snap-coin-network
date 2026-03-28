// File: static/app.js
// Project: snap-coin-network / static/
// Version: 0.1.3
// Description: WebSocket client, world map rendering, peer dots, laser lines, zoom/pan

'use strict';

const WS_URL      = `wss://${location.host}/ws`;
const API_SUMMARY = '/api/summary';
const API_PEERS   = '/api/peers';

let SELF     = { lat: 20, lon: 0 };
let peers    = {};
let ws       = null;
let mapReady = false;

const MAP_W = 1010;
const MAP_H = 665;

// ── Zoom/Pan state ────────────────────────────────────────────────────────────
let viewX    = 0;
let viewY    = 0;
let viewZoom = 1;
let isPanning = false;
let panStart  = { x: 0, y: 0 };

function project(lat, lon) {
  const x = (lon + 180) * (MAP_W / 360);
  const y = (90 - lat)  * (MAP_H / 180);
  return { x, y };
}

function applyTransform() {
  const g = document.getElementById('map-root');
  if (g) g.setAttribute('transform', `translate(${viewX},${viewY}) scale(${viewZoom})`);
}

function initZoomPan() {
  const svg = document.getElementById('world-map');

  svg.addEventListener('wheel', (e) => {
    e.preventDefault();
    const rect    = svg.getBoundingClientRect();
    const mouseX  = e.clientX - rect.left;
    const mouseY  = e.clientY - rect.top;
    const svgX    = (mouseX / rect.width)  * MAP_W;
    const svgY    = (mouseY / rect.height) * MAP_H;

    const delta   = e.deltaY > 0 ? 0.85 : 1.18;
    const newZoom = Math.min(Math.max(viewZoom * delta, 0.8), 12);

    viewX = svgX - (svgX - viewX) * (newZoom / viewZoom);
    viewY = svgY - (svgY - viewY) * (newZoom / viewZoom);
    viewZoom = newZoom;
    applyTransform();
  }, { passive: false });

  svg.addEventListener('mousedown', (e) => {
    isPanning = true;
    panStart  = { x: e.clientX - viewX, y: e.clientY - viewY };
    svg.style.cursor = 'grabbing';
  });

  window.addEventListener('mousemove', (e) => {
    if (!isPanning) return;
    viewX = e.clientX - panStart.x;
    viewY = e.clientY - panStart.y;
    applyTransform();
  });

  window.addEventListener('mouseup', () => {
    isPanning = false;
    svg.style.cursor = 'grab';
  });

  svg.style.cursor = 'grab';

  // Double-click to reset
  svg.addEventListener('dblclick', () => {
    viewX    = 0;
    viewY    = 0;
    viewZoom = 1;
    applyTransform();
  });
}

async function loadMap() {
  const svg        = document.getElementById('world-map');
  const countriesG = document.getElementById('countries');

  try {
    const res  = await fetch('/world.geojson');
    const data = await res.json();
    data.features.forEach(feature => {
      const geom = feature.geometry;
      if (!geom) return;
      const polys = geom.type === 'Polygon'
        ? [geom.coordinates]
        : geom.type === 'MultiPolygon'
          ? geom.coordinates
          : [];
      polys.forEach(poly => {
        poly.forEach(ring => {
          const d = ring.map(([lon, lat], i) => {
            const { x, y } = project(lat, lon);
            return `${i === 0 ? 'M' : 'L'}${x.toFixed(2)},${y.toFixed(2)}`;
          }).join(' ') + ' Z';
          const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
          path.setAttribute('d', d);
          countriesG.appendChild(path);
        });
      });
    });
  } catch (e) {
    console.warn('World map not loaded:', e);
  }

  initZoomPan();
  mapReady = true;
  redrawPeers();
}

function redrawPeers() {
  if (!mapReady) return;

  document.querySelectorAll('.peer-dot').forEach(el => el.remove());
  document.querySelectorAll('.laser').forEach(el => el.remove());

  const svg     = document.getElementById('world-map');
  const lasersG = document.getElementById('laser-lines');
  const selfPos = project(SELF.lat, SELF.lon);

  const selfDot = document.getElementById('self-dot');
  selfDot.setAttribute('cx', selfPos.x);
  selfDot.setAttribute('cy', selfPos.y);

  Object.values(peers).forEach(peer => {
    if (peer.lat == null || peer.lon == null) return;

    // Small jitter to separate overlapping dots
    const jitter = 2;
    const jx     = (Math.random() - 0.5) * jitter;
    const jy     = (Math.random() - 0.5) * jitter;
    const pos    = project(peer.lat, peer.lon);
    const px     = pos.x + jx;
    const py     = pos.y + jy;

    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('x1', selfPos.x);
    line.setAttribute('y1', selfPos.y);
    line.setAttribute('x2', px);
    line.setAttribute('y2', py);
    line.classList.add('laser', peer.inbound ? 'inbound' : 'outbound');
    line.dataset.addr = peer.addr;
    lasersG.appendChild(line);

    const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    dot.setAttribute('cx', px);
    dot.setAttribute('cy', py);
    dot.setAttribute('r', 4);
    dot.setAttribute('fill', peer.inbound ? '#00c8a0' : '#d4900a');
    dot.setAttribute('opacity', '0.85');
    dot.classList.add('peer-dot');
    dot.dataset.addr = peer.addr;

    const title = document.createElementNS('http://www.w3.org/2000/svg', 'title');
    title.textContent = `${peer.addr}\n${peer.city || ''}${peer.city ? ', ' : ''}${peer.country || 'Unknown'}`;
    dot.appendChild(title);
    svg.appendChild(dot);
  });
}

function pulseLasers(type) {
  const lasersG  = document.getElementById('laser-lines');
  const selfPos  = project(SELF.lat, SELF.lon);
  const cssClass = type === 'block_seen' ? 'block-pulse' : 'tx-pulse';

  Object.values(peers).forEach(peer => {
    if (peer.lat == null || peer.lon == null) return;
    const pos  = project(peer.lat, peer.lon);
    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('x1', selfPos.x);
    line.setAttribute('y1', selfPos.y);
    line.setAttribute('x2', pos.x);
    line.setAttribute('y2', pos.y);
    line.classList.add('laser', 'pulse', cssClass);
    lasersG.appendChild(line);
    setTimeout(() => line.remove(), 700);
  });
}

function renderPeerTable() {
  const tbody = document.getElementById('peer-tbody');
  const count = document.getElementById('peer-count');
  const list  = Object.values(peers);
  count.textContent = list.length;

  tbody.innerHTML = list.map(p => {
    const age      = elapsed(p.connected_at);
    const location = [p.city, p.country].filter(Boolean).join(', ') || '—';
    const dir      = p.inbound
      ? '<span class="dir-badge in">IN</span>'
      : '<span class="dir-badge out">OUT</span>';
    return `<tr>
      <td>${p.addr}</td>
      <td>${dir}</td>
      <td>${location}</td>
      <td>${age}</td>
    </tr>`;
  }).join('');
}

function pushEvent(icon, text) {
  const feed = document.getElementById('event-feed');
  const row  = document.createElement('div');
  row.className = 'event-row';
  row.innerHTML = `
    <span class="event-icon">${icon}</span>
    <span class="event-text">${text}</span>
    <span class="event-time">${timeNow()}</span>
  `;
  feed.prepend(row);
  while (feed.children.length > 80) feed.lastChild.remove();
}

function connect() {
  ws = new WebSocket(WS_URL);
  ws.onopen = () => {
    setWsStatus(true);
    fetchSummary();
    fetchPeers();
  };
  ws.onclose = () => {
    setWsStatus(false);
    setTimeout(connect, 3000);
  };
  ws.onmessage = (e) => {
    let event;
    try { event = JSON.parse(e.data); } catch { return; }
    handleEvent(event);
  };
}

function handleEvent(event) {
  switch (event.type) {
    case 'peer_connected': {
      peers[event.addr] = {
        addr:         event.addr,
        inbound:      event.inbound,
        connected_at: event.timestamp,
        lat:          null,
        lon:          null,
        country:      null,
        city:         null,
      };
      fetchPeers();
      pushEvent('🟢', `${event.inbound ? '↙ IN' : '↗ OUT'} ${event.addr}`);
      renderPeerTable();
      redrawPeers();
      break;
    }
    case 'peer_disconnected': {
      delete peers[event.addr];
      pushEvent('🔴', `DISC ${event.addr}`);
      renderPeerTable();
      redrawPeers();
      break;
    }
    case 'block_seen': {
      pushEvent('⬡', 'Block propagated');
      pulseLasers('block_seen');
      fetchSummary();
      break;
    }
    case 'transaction_seen': {
      pushEvent('→', 'Transaction seen');
      pulseLasers('tx_seen');
      fetchSummary();
      break;
    }
  }
}

async function fetchSummary() {
  try {
    const res  = await fetch(API_SUMMARY);
    const data = await res.json();
    document.getElementById('stat-peers').textContent       = data.active_peers;
    document.getElementById('stat-connections').textContent = data.total_connections;
    document.getElementById('stat-blocks').textContent      = data.blocks_seen;
    document.getElementById('stat-txs').textContent         = data.transactions_seen;
    document.getElementById('stat-uptime').textContent      = elapsed(data.uptime_since);

    if (data.self && data.self.lat != null) {
      SELF = { lat: data.self.lat, lon: data.self.lon };
      redrawPeers();
    }
  } catch {}
}

async function fetchPeers() {
  try {
    const res  = await fetch(API_PEERS);
    const data = await res.json();
    data.peers.forEach(p => {
      peers[p.addr] = {
        addr:         p.addr,
        inbound:      p.inbound,
        connected_at: p.connected_at,
        lat:          p.lat,
        lon:          p.lon,
        country:      p.country,
        city:         p.city,
      };
    });
    renderPeerTable();
    redrawPeers();
  } catch {}
}

function elapsed(unix) {
  const secs = Math.floor(Date.now() / 1000) - unix;
  if (secs < 60)   return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

function timeNow() {
  return new Date().toTimeString().slice(0, 8);
}

function setWsStatus(ok) {
  const dot   = document.getElementById('ws-dot');
  const label = document.getElementById('ws-label');
  dot.className     = 'ws-dot ' + (ok ? 'connected' : 'disconnected');
  label.textContent = ok ? 'live' : 'reconnecting';
}

setInterval(fetchSummary, 10000);
setInterval(renderPeerTable, 5000);

loadMap();
connect();

// File: static/app.js / snap-coin-network / 2026-03-27