// Mapist renderer.
//
// Draws the snapshot the Rust side sends and routes user input back as
// commands. All map semantics live in Rust; this file is presentation and
// input only.

const invoke = window.__TAURI__?.core?.invoke;

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const els = {
  status: document.getElementById("status"),
  text: document.getElementById("node-text"),
  note: document.getElementById("node-note"),
  meta: document.getElementById("node-meta"),
  search: document.getElementById("search"),
  btnNew: document.getElementById("btn-new"),
  btnOpen: document.getElementById("btn-open"),
  btnSave: document.getElementById("btn-save"),
  btnAdd: document.getElementById("btn-add"),
  btnRemove: document.getElementById("btn-remove"),
  btnUndo: document.getElementById("btn-undo"),
  btnRedo: document.getElementById("btn-redo"),
  btnFold: document.getElementById("btn-fold"),
  btnColor: document.getElementById("btn-color"),
};

const PALETTE = [
  null,
  "#c6262e",
  "#f37329",
  "#f9c440",
  "#68b723",
  "#3689e6",
  "#a56de2",
];

let snap = null;
let selected = null;
let hovered = null;
let matches = new Set();

// Viewport transform.
const view = { x: 0, y: 0, scale: 1 };
let dragging = null;

function css(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.floor(rect.width * dpr));
  canvas.height = Math.max(1, Math.floor(rect.height * dpr));
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  draw();
}

function toScreen(x, y) {
  return { x: x * view.scale + view.x, y: y * view.scale + view.y };
}

function toWorld(sx, sy) {
  return { x: (sx - view.x) / view.scale, y: (sy - view.y) / view.scale };
}

function nodeById(id) {
  return snap?.nodes.find((n) => n.id === id) ?? null;
}

function fitView() {
  if (!snap || snap.nodes.length === 0) return;
  const b = snap.bounds;
  const pad = 60;
  const rect = canvas.getBoundingClientRect();
  const w = b.max_x - b.min_x || 1;
  const h = b.max_y - b.min_y || 1;
  const scale = Math.min((rect.width - pad * 2) / w, (rect.height - pad * 2) / h, 1.6);
  view.scale = Math.max(0.15, scale);
  view.x = rect.width / 2 - ((b.min_x + b.max_x) / 2) * view.scale;
  view.y = rect.height / 2 - ((b.min_y + b.max_y) / 2) * view.scale;
}

// ── drawing ────────────────────────────────────────────────────────────────

function draw() {
  const rect = canvas.getBoundingClientRect();
  ctx.clearRect(0, 0, rect.width, rect.height);
  if (!snap || snap.nodes.length === 0) {
    ctx.fillStyle = css("--text-dim");
    ctx.font = "14px -apple-system, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("No map open — press New to start", rect.width / 2, rect.height / 2);
    ctx.textAlign = "start";
    return;
  }

  const color = css("--text");
  const line = css("--line");
  const accent = css("--accent");

  // Branches first, so nodes paint over their connectors.
  for (const n of snap.nodes) {
    if (n.parent === null) continue;
    const p = nodeById(n.parent);
    if (!p) continue;
    drawBranch(p, n, n.color || line);
  }

  for (const c of snap.connections) {
    const a = nodeById(c.from);
    const b = nodeById(c.to);
    if (a && b) drawConnection(a, b, c.color || accent);
  }

  for (const n of snap.nodes) drawNode(n, color, line, accent);
}

function drawBranch(parent, child, stroke) {
  const a = toScreen(parent.x + parent.width / 2, parent.y + parent.height / 2);
  const b = toScreen(child.x + child.width / 2, child.y + child.height / 2);
  const fromX = child.side === "left" ? parent.x : parent.x + parent.width;
  const p = toScreen(fromX, parent.y + parent.height / 2);

  ctx.strokeStyle = stroke;
  ctx.lineWidth = Math.max(1.2, 2.4 * view.scale);
  ctx.beginPath();
  ctx.moveTo(p.x, p.y);
  const midX = (p.x + b.x) / 2;
  ctx.bezierCurveTo(midX, p.y, midX, b.y, b.x, b.y);
  ctx.stroke();
}

function drawConnection(a, b, stroke) {
  const p = toScreen(a.x + a.width / 2, a.y + a.height / 2);
  const q = toScreen(b.x + b.width / 2, b.y + b.height / 2);
  ctx.save();
  ctx.strokeStyle = stroke;
  ctx.lineWidth = 1.6;
  ctx.setLineDash([5, 4]);
  ctx.beginPath();
  ctx.moveTo(p.x, p.y);
  ctx.lineTo(q.x, q.y);
  ctx.stroke();
  ctx.restore();
}

function drawNode(n, color, line, accent) {
  const p = toScreen(n.x, n.y);
  const w = n.width * view.scale;
  const h = n.height * view.scale;
  const isSel = n.id === selected;
  const isHover = n.id === hovered;
  const isMatch = matches.has(n.id);

  // Box.
  const radius = Math.min(6, h / 3);
  ctx.beginPath();
  ctx.roundRect(p.x, p.y, w, h, radius);
  ctx.fillStyle = css("--panel-2");
  ctx.fill();

  if (n.color) {
    ctx.fillStyle = n.color;
    ctx.globalAlpha = 0.22;
    ctx.fill();
    ctx.globalAlpha = 1;
  }

  ctx.lineWidth = isSel ? 2.5 : isMatch ? 2 : 1.2;
  ctx.strokeStyle = isSel ? accent : isMatch ? "#f9c440" : isHover ? accent : n.color || line;
  ctx.stroke();

  // Text.
  const fontSize = Math.max(8, 12 * view.scale);
  ctx.font = `${fontSize}px -apple-system, sans-serif`;
  ctx.fillStyle = color;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const label = truncate(n.text, w - 12, ctx);
  ctx.fillText(label, p.x + w / 2, p.y + h / 2);

  // Fold marker.
  if (n.has_children && n.folded) {
    ctx.beginPath();
    ctx.arc(p.x + w + 5, p.y + h / 2, 4, 0, Math.PI * 2);
    ctx.fillStyle = accent;
    ctx.fill();
  }

  ctx.textAlign = "start";
  ctx.textBaseline = "alphabetic";
}

function truncate(text, maxWidth, context) {
  if (context.measureText(text).width <= maxWidth) return text;
  let out = text;
  while (out.length > 1 && context.measureText(out + "…").width > maxWidth) {
    out = out.slice(0, -1);
  }
  return out + "…";
}

// ── hit testing ────────────────────────────────────────────────────────────

function hitTest(clientX, clientY) {
  if (!snap) return null;
  const rect = canvas.getBoundingClientRect();
  const w = toWorld(clientX - rect.left, clientY - rect.top);
  // Reverse so the topmost node wins.
  for (let i = snap.nodes.length - 1; i >= 0; i--) {
    const n = snap.nodes[i];
    if (w.x >= n.x && w.x <= n.x + n.width && w.y >= n.y && w.y <= n.y + n.height) {
      return n;
    }
  }
  return null;
}

// ── state sync ─────────────────────────────────────────────────────────────

function apply(next) {
  snap = next;
  if (selected !== null && !nodeById(selected)) selected = null;
  if (selected === null && snap.nodes.length > 0) selected = snap.nodes[0].id;
  syncInspector();
  syncStatus();
  draw();
}

function syncInspector() {
  const n = selected !== null ? nodeById(selected) : null;
  els.text.disabled = !n;
  els.note.disabled = !n;
  els.text.value = n ? n.text : "";
  els.note.value = n ? n.note : "";
  els.meta.textContent = n
    ? `id ${n.id} · depth ${n.depth} · ${n.side} · ${Math.round(n.width)}×${Math.round(n.height)}`
    : "no selection";
}

function syncStatus() {
  if (!snap) {
    els.status.textContent = "";
    return;
  }
  const name = snap.path ? snap.path.split("/").pop() : "untitled";
  const dirty = snap.dirty ? " •" : "";
  els.status.textContent = `${name}${dirty} — ${snap.nodes.length} nodes`;
  els.btnUndo.disabled = !snap.can_undo;
  els.btnRedo.disabled = !snap.can_redo;
}

async function run(cmd, args = {}) {
  if (!invoke) {
    console.error("Tauri bridge unavailable");
    return;
  }
  try {
    apply(await invoke(cmd, args));
  } catch (err) {
    console.error(cmd, err);
    els.status.textContent = String(err);
  }
}

// ── actions ────────────────────────────────────────────────────────────────

const ACTIONS = {
  new: () => run("new_map", { title: "Central Topic" }),
  addChild: () => {
    if (selected === null) return;
    run("add_node", { parent: selected, text: "New idea" });
  },
  addSibling: () => {
    const n = nodeById(selected);
    if (!n || n.parent === null) return;
    run("add_node", { parent: n.parent, text: "New idea" });
  },
  remove: () => {
    const n = nodeById(selected);
    if (!n || n.parent === null) return;
    run("remove_node", { id: selected });
  },
  fold: () => {
    if (selected === null) return;
    run("toggle_fold", { id: selected });
  },
  colour: () => {
    const n = nodeById(selected);
    if (!n) return;
    const i = PALETTE.indexOf(n.color);
    const next = PALETTE[(i + 1) % PALETTE.length];
    run("set_color", { id: selected, color: next });
  },
  undo: () => run("undo"),
  redo: () => run("redo"),
};

function moveSelection(dx, dy) {
  if (selected === null || !snap) return;
  const cur = nodeById(selected);
  if (!cur) return;
  const cx = cur.x + cur.width / 2;
  const cy = cur.y + cur.height / 2;
  let best = null;
  let bestScore = Infinity;
  for (const n of snap.nodes) {
    if (n.id === cur.id) continue;
    const nx = n.x + n.width / 2;
    const ny = n.y + n.height / 2;
    const ddx = nx - cx;
    const ddy = ny - cy;
    if (dx !== 0 && Math.sign(ddx) !== Math.sign(dx)) continue;
    if (dy !== 0 && Math.sign(ddy) !== Math.sign(dy)) continue;
    const along = dx !== 0 ? Math.abs(ddx) : Math.abs(ddy);
    const across = dx !== 0 ? Math.abs(ddy) : Math.abs(ddx);
    const score = along + across * 3;
    if (score < bestScore) {
      bestScore = score;
      best = n;
    }
  }
  if (best) {
    selected = best.id;
    syncInspector();
    draw();
  }
}

// ── events ─────────────────────────────────────────────────────────────────

els.btnNew.onclick = ACTIONS.new;
els.btnOpen.onclick = async () => {
  const path = window.prompt("Path to .minder file");
  if (path) run("open_map", { path });
};
els.btnSave.onclick = () => {
  const path = snap?.path || window.prompt("Save as (path)");
  if (path) run("save_map", { path });
};
els.btnAdd.onclick = ACTIONS.addChild;
els.btnRemove.onclick = ACTIONS.remove;
els.btnUndo.onclick = ACTIONS.undo;
els.btnRedo.onclick = ACTIONS.redo;
els.btnFold.onclick = ACTIONS.fold;
els.btnColor.onclick = ACTIONS.colour;

els.text.oninput = () => {
  if (selected === null) return;
  clearTimeout(els.text._t);
  els.text._t = setTimeout(
    () => run("set_text", { id: selected, text: els.text.value }),
    350
  );
};

els.note.oninput = () => {
  if (selected === null) return;
  clearTimeout(els.note._t);
  els.note._t = setTimeout(
    () => run("set_note", { id: selected, note: els.note.value }),
    450
  );
};

els.search.oninput = async () => {
  const q = els.search.value.trim();
  if (!q || !invoke) {
    matches = new Set();
    draw();
    return;
  }
  try {
    matches = new Set(await invoke("search_nodes", { query: q, limit: 200 }));
    draw();
  } catch (err) {
    console.error(err);
  }
};

canvas.addEventListener("mousedown", (e) => {
  const hit = hitTest(e.clientX, e.clientY);
  if (hit) {
    selected = hit.id;
    syncInspector();
    draw();
    return;
  }
  dragging = { x: e.clientX - view.x, y: e.clientY - view.y };
  canvas.classList.add("dragging");
});

window.addEventListener("mousemove", (e) => {
  if (dragging) {
    view.x = e.clientX - dragging.x;
    view.y = e.clientY - dragging.y;
    draw();
    return;
  }
  const hit = hitTest(e.clientX, e.clientY);
  const id = hit ? hit.id : null;
  if (id !== hovered) {
    hovered = id;
    draw();
  }
});

window.addEventListener("mouseup", () => {
  dragging = null;
  canvas.classList.remove("dragging");
});

canvas.addEventListener("dblclick", (e) => {
  const hit = hitTest(e.clientX, e.clientY);
  if (hit) {
    selected = hit.id;
    syncInspector();
    els.text.focus();
    els.text.select();
  }
});

canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const before = toWorld(mx, my);
    const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    view.scale = Math.min(4, Math.max(0.1, view.scale * factor));
    const after = toWorld(mx, my);
    view.x += (after.x - before.x) * view.scale;
    view.y += (after.y - before.y) * view.scale;
    draw();
  },
  { passive: false }
);

window.addEventListener("keydown", (e) => {
  const inField = e.target.tagName === "TEXTAREA" || e.target.tagName === "INPUT";
  const mod = e.metaKey || e.ctrlKey;

  if (mod && e.key.toLowerCase() === "z") {
    e.preventDefault();
    e.shiftKey ? ACTIONS.redo() : ACTIONS.undo();
    return;
  }
  if (mod && e.key.toLowerCase() === "s") {
    e.preventDefault();
    els.btnSave.onclick();
    return;
  }
  if (inField) return;

  switch (e.key) {
    case "Tab":
      e.preventDefault();
      ACTIONS.addChild();
      break;
    case "Enter":
      e.preventDefault();
      ACTIONS.addSibling();
      break;
    case "Backspace":
    case "Delete":
      e.preventDefault();
      ACTIONS.remove();
      break;
    case " ":
      e.preventDefault();
      ACTIONS.fold();
      break;
    case "ArrowUp":
      e.preventDefault();
      moveSelection(0, -1);
      break;
    case "ArrowDown":
      e.preventDefault();
      moveSelection(0, 1);
      break;
    case "ArrowLeft":
      e.preventDefault();
      moveSelection(-1, 0);
      break;
    case "ArrowRight":
      e.preventDefault();
      moveSelection(1, 0);
      break;
    default:
      break;
  }
});

window.addEventListener("resize", resize);

// ── boot ───────────────────────────────────────────────────────────────────

if (invoke) {
  invoke("get_snapshot").then((s) => {
    if (s && s.nodes.length > 0) {
      apply(s);
      fitView();
      draw();
    } else {
      run("new_map", { title: "Central Topic" }).then(() => {
        fitView();
        draw();
      });
    }
  });
} else {
  els.status.textContent = "running outside Tauri";
}

resize();
