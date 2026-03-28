// =============================================================================
// File: static/app.js
// Project: snap-coin-network / static/
// Version: 0.2.0
// Description: Leaflet map, WebSocket client, peer markers, laser lines,
//              peer table, live event feed. Replaces SVG map with Leaflet
//              for cross-browser/mobile compatibility.
// Modified: 2026-03-28
// =============================================================================

'use strict';

const WS_URL      = `wss://${location.host}/ws`;
const API_SUMMARY = '/api/summary';
const API_PEERS   = '/api/peers';

let SELF  = { lat: 20, lon: 0 };
let peers = {};
let ws    = null;

// ── Leaflet map setup ─────────────────────────────────────────────────────────
const map = L.map('world-map', {
  center: [20, 0],
  zoom: 2,
  minZoom: 1,
  maxZoom: 10,
  zoomControl: true,
  attributionControl: false,
  worldCopyJump: false,
});

L.tileLayer('https://{s}.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}{r}.png', {
  subdomains: 'abcd',
  maxZoom: 19,
}).addTo(map);

// ── Marker + line layers ──────────────────────────────────────────────────────
const linesLayer   = L.layerGroup().addTo(map);
const markersLayer = L.layerGroup().addTo(map);
const pulseLayer   = L.layerGroup().addTo(map);

// Self marker
const selfIcon = L.divIcon({
  className: '',
  html: '<div class="self-marker"></div>',
  iconSize: [12, 12],
  iconAnchor: [6, 6],
});
let selfMarker = L.marker([SELF.lat, SELF.lon], { icon: selfIcon }).addTo(map);

// ── Peer dot icon factory ─────────────────────────────────────────────────────
function peerIcon(inbound) {
  const color = inbound ? '#00c8a0' : '#d4900a';
  return L.divIcon({
    className: '',
    html: `<div class="peer-marker" style="background:${color};box-shadow:0 0 5px ${color}"></div>`,
    iconSize: [10, 10],
    iconAnchor: [5, 5],
  });
}

// ── Redraw all peer markers and lines ─────────────────────────────────────────
function redrawPeers() {
  linesLayer.clearLayers();
  markersLayer.clearLayers();

  const selfLatLng = [SELF.lat, SELF.lon];

  Object.values(peers).forEach(peer => {
    if (peer.lat == null || peer.lon == null) return;

    const peerLatLng = [peer.lat, peer.lon];

    // Line from self to peer
    L.polyline([selfLatLng, peerLatLng], {
      color:   peer.inbound ? '#00c8a0' : '#d4900a',
      weight:  1,
      opacity: 0.25,
    }).addTo(linesLayer);

    // Peer dot
    const marker = L.marker(peerLatLng, { icon: peerIcon(peer.inbound) });
    const location = [peer.city, peer.country].filter(Boolean).join(', ') || 'Unknown';
    marker.bindTooltip(`${peer.addr}<br>${location}`, {
      className: 'peer-tooltip',
      direction: 'top',
      offset: [0, -6],
    });
    marker.addTo(markersLayer);
  });
}

// ── Pulse animation on block/tx ───────────────────────────────────────────────
function pulseLasers(type) {
  const color    = type === 'block_seen' ? '#00ffc8' : '#d4900a';
  const selfLatLng = [SELF.lat, SELF.lon];

  Object.values(peers).forEach(peer => {
    if (peer.lat == null || peer.lon == null) return;
    const line = L.polyline([[peer.lat, peer.lon], selfLatLng], {
      color,
      weight:  2,
      opacity: 0.9,
      dashArray: '8 4',
    }).addTo(pulseLayer);
    setTimeout(() => pulseLayer.removeLayer(line), 700);
  });
}

// ── Peer table ────────────────────────────────────────────────────────────────
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

// ── Event feed ────────────────────────────────────────────────────────────────
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

// ── WebSocket ─────────────────────────────────────────────────────────────────
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

// ── API fetches ───────────────────────────────────────────────────────────────
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
      selfMarker.setLatLng([SELF.lat, SELF.lon]);
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

// ── Helpers ───────────────────────────────────────────────────────────────────
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

// ── Init ──────────────────────────────────────────────────────────────────────
setInterval(fetchSummary, 10000);
setInterval(renderPeerTable, 5000);

connect();

// =============================================================================
// File: static/app.js
// Project: snap-coin-network / static/
// Created: 2026-03-28T00:00:00Z
// =============================================================================