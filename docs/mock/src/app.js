/* michi screen mock: the page script. build.py puts DATA (the real tsumugi
   history, gen_data.py) and FAKE (the made-up Claude sessions, fake.py) in
   front of it. */
(() => {
'use strict';

/* ── helpers ──────────────────────────────────────────────── */
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const esc = s => String(s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const sum = (files, k) => files.reduce((s, f) => s + Math.max(0, f[k]), 0);
const leaf = p => p.split('/').pop();
const store = {
  get(k, d) { try { const v = localStorage.getItem('michi-mock.' + k); return v == null ? d : v; } catch (e) { return d; } },
  set(k, v) { try { localStorage.setItem('michi-mock.' + k, v); } catch (e) { /* storage blocked: the page works without it */ } },
};

const p2 = n => String(n).padStart(2, '0');
const jst = t => new Date((t + 9 * 3600) * 1000);
const hm = t => { const d = jst(t); return p2(d.getUTCHours()) + ':' + p2(d.getUTCMinutes()); };
const fullTime = t => { const d = jst(t); return `${d.getUTCFullYear()}/${p2(d.getUTCMonth() + 1)}/${p2(d.getUTCDate())} ${hm(t)}`; };
function when(t) {
  const day = Math.floor((t - FAKE.day) / 86400);
  if (day === 0) return hm(t);
  if (day === -1) return '昨日 ' + hm(t);
  const d = jst(t);
  return `${d.getUTCMonth() + 1}/${d.getUTCDate()} ${hm(t)}`;
}

/* ── themes (ito-theme order) ─────────────────────────────── */
const THEME_KEYS = ['bg', 'side', 'panel', 'border', 'fg', 'dim', 'wait', 'run', 'err', 'done', 'blue', 'magenta'];
const THEMES = {
  'tsumugi Dark': [false, '16181d 121418 1b1e24 2c3039 c8cdd8 9aa3b5 e8c87a 6fd0d0 f07178 8ed08e 7ab8f5 c99cf0'],
  'tsumugi Light': [true, 'f7f8fa eef0f3 ffffff dde1e7 1f2430 5a6272 b07c1a 137a7a c43642 2e7d3a 2a5db0 8a3fb5'],
  'Tokyo Night': [false, '1a1b26 16161e 1f2335 292e42 c0caf5 9aa5ce e0af68 7dcfff f7768e 9ece6a 7aa2f7 bb9af7'],
  'Nord': [false, '2e3440 272c36 3b4252 434c5e eceff4 b8c1d1 ebcb8b 88c0d0 d08770 a3be8c 81a1c1 b48ead'],
};
const rgb = h => [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16));
const mix = (a, b, t) => '#' + rgb(a).map((x, i) => Math.round(x + (rgb(b)[i] - x) * t).toString(16).padStart(2, '0')).join('');

function applyTheme() {
  const [light, list] = THEMES[S.theme];
  const c = {};
  list.split(' ').forEach((v, i) => { c[THEME_KEYS[i]] = v; });
  const win = $('#win');
  for (const k of THEME_KEYS) win.style.setProperty('--t-' + k, '#' + c[k]);
  win.style.setProperty('--t-sel', mix(c.panel, c.run, light ? 0.18 : 0.25));
  win.style.setProperty('--t-on', '#' + c.bg);
  // ANSI 16: the normal eight from the theme, the bright eight a quarter of the way to white (to black on a light theme).
  const normal = [c.panel, c.err, c.done, c.wait, c.blue, c.magenta, c.run, c.fg];
  normal.forEach((v, i) => {
    win.style.setProperty('--a' + i, '#' + v);
    win.style.setProperty('--a' + (i + 8), mix(v, light ? '000000' : 'ffffff', 0.25));
  });
}

/* Lane colors come from ANSI 16 without red (1, 9) and the grays (0, 7, 8, 15). */
const NAMED = { 'main': 6, 'origin/main': 14, 'fix/142-optional-locks': 3, 'feat/new-worktree': 5, 'feat/focus': 2 };
const CYCLE = [4, 12, 13, 10, 11];
function laneColor(name) {
  if (name == null) return 'var(--a12)';
  if (name in NAMED) return `var(--a${NAMED[name]})`;
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return `var(--a${CYCLE[h % CYCLE.length]})`;
}

/* ── logos ────────────────────────────────────────────────── */
const LOGOS = {
  A: {
    name: 'A 合流',
    note: '2 本の線が 1 本になる。マージと、worktree の作業を main に戻すことの印。樹形図と同じ形なので、画面とつながる。',
    body: '<path d="M10 4v24" fill="none" style="stroke:var(--lc)" stroke-width="3" stroke-linecap="round"/>'
      + '<path d="M22 25v-5c0-6-12-5-12-11" fill="none" style="stroke:var(--lg)" stroke-width="3" stroke-linecap="round"/>'
      + '<circle cx="22" cy="25" r="3.6" style="fill:var(--lbg);stroke:var(--lg)" stroke-width="2.4"/>'
      + '<circle cx="10" cy="7" r="3.6" style="fill:var(--lc)"/>',
  },
  B: {
    name: 'B 道標',
    note: '分かれ道の標識。どの worktree で何が進んでいるかを見て、行き先を決める道具、という読み。16px では板が塊に見えやすい。',
    body: '<path d="M15 4v24" style="stroke:var(--lc)" stroke-width="2.6" stroke-linecap="round"/>'
      + '<path d="M7 7h15l4 4-4 4H7z" style="fill:var(--lc)"/>'
      + '<path d="M25 17H10l-4 4 4 4h15z" style="fill:var(--lg)"/>',
  },
  C: {
    name: 'C 道',
    note: '遠くへ続く 1 本の道。名前（michi）にいちばん素直。Git の道具らしさは薄い。',
    body: '<path d="M4 28L13 4M28 28L19 4" fill="none" style="stroke:var(--lc)" stroke-width="2.8" stroke-linecap="round"/>'
      + '<path d="M16 27v-5M16 17v-3.5M16 10v-2" style="stroke:var(--lg)" stroke-width="2.6" stroke-linecap="round"/>',
  },
};
const LOGO = 'A';  // chosen on 2026-10-15
const logoSvg = (k, cls = '') => `<svg viewBox="0 0 32 32"${cls ? ` class="${cls}"` : ''} aria-hidden="true">${LOGOS[k].body}</svg>`;
// The app icon: assets/icon.svg (the mark on tsumugi Dark's tile), put in by build.py.
const iconSvg = cls => ICON.replace(/^<svg /, `<svg class="${cls}" `).replace(/ role="img" aria-label="michi"/, ' aria-hidden="true"');

/* ── state ────────────────────────────────────────────────── */
const SCENES = ['graph', 'wt', 'cf', 'logo'];
const S = {
  scene: 'graph', width: '1280', theme: store.get('theme', 'tsumugi Dark'), fit: 'fit',
  view: 'graph', merging: false, sel: 0, wide: false, filter: '', limit: 64, open: { feat: true, fix: true },
  wtSel: 1, k: 1, cf: null, mx: false,
  // Fold the merges on main's first-parent line (remembered; the app keeps it per repository).
  fold: store.get('fold', '1') !== '0', unfold: new Set(),
};
if (!THEMES[S.theme]) S.theme = 'tsumugi Dark';
const narrow = () => S.width === '960';

const WT = FAKE.worktrees;
const wtByBranch = Object.fromEntries(WT.map(w => [w.branch, w]));
const COMMIT = {};
for (const c of [...FAKE.commits, ...DATA.commits, FAKE.merged]) COMMIT[c.h] = c;
const CB = FAKE.conflict.blocks;

const CL = { run: ['●', '実行中', 'st-run'], wait: ['◐', '待ち', 'st-wait'], done: ['✓', '済み', 'st-done'], err: ['✕', '止まった', 'st-err'] };
const claudeDot = c => `<span class="${CL[c.state][2]}" title="Claude ${CL[c.state][1]}（タブ ${c.tab}）">${CL[c.state][0]}</span>`;

const PREDICT_HEADS = {};
for (const [k, v] of Object.entries(FAKE.predict)) {
  const [a, b] = k.split('|');
  const n = v.reduce((s, x) => s + x[1], 0), files = v.map(x => leaf(x[0])).join('・');
  PREDICT_HEADS[wtByBranch[a].head] = `${b} と ${n} か所でぶつかる見込み（${files}）`;
  PREDICT_HEADS[wtByBranch[b].head] = `${a} と ${n} か所でぶつかる見込み（${files}）`;
}
/* Pairs that only collide once the changes not yet committed are counted:
   michi writes a tree of each worktree's files (refs untouched) and merges
   those. The first branch of the key is the one whose WIP does it. */
const PREDICT_WIP = {
  'fix/142-optional-locks|feat/new-worktree': [['CHANGELOG.md', 1], ['Cargo.toml', 1]],
  'fix/142-optional-locks|feat/focus': [['CHANGELOG.md', 1], ['Cargo.toml', 1]],
};
const WIP_HITS = {};
for (const [k, v] of Object.entries(PREDICT_WIP)) {
  const [a, b] = k.split('|');
  (WIP_HITS[a] ||= []).push(`${b} と ${v.reduce((s, x) => s + x[1], 0)} か所（${v.map(x => leaf(x[0])).join('・')}）`);
}
function predictPair(a, b) {
  for (const [m, wip] of [[FAKE.predict, false], [PREDICT_WIP, true]]) {
    for (const k of [a + '|' + b, b + '|' + a]) if (m[k]) return { k, v: m[k], wip };
  }
  return null;
}

/* Commits nobody has looked at yet: the newest ones of each worktree. */
const UNSEEN = {};
for (const w of WT) {
  if (!w.unseen) continue;
  let h = w.head;
  for (let k = 0; k < w.unseen.commits && h && COMMIT[h]; k++) { UNSEEN[h] = w.branch; h = COMMIT[h].p[0]; }
}
const seen = new Set();
const isUnseen = h => h in UNSEEN && !seen.has(h);
const unseenOf = b => Object.keys(UNSEEN).filter(h => UNSEEN[h] === b && !seen.has(h));

function heads() {
  const m = {};
  for (const w of WT) m[w.branch] = w.head;
  if (S.merging) m.main = FAKE.merged.h;
  return m;
}
function wtOfHead(h) {
  const hs = heads();
  return Object.keys(hs).find(b => hs[b] === h) || null;
}

/* ── refs and pills ───────────────────────────────────────── */
function refsOf(c) {
  let r = c.r;
  if (S.merging && c.h === 'bb3e879') r = r.filter(x => x !== 'HEAD -> main');
  const out = { locals: [], remotes: [], tags: [] };
  for (let x of r) {
    if (x === 'origin/HEAD') continue;
    if (x.startsWith('tag: ')) { out.tags.push(x.slice(5)); continue; }
    if (x.startsWith('HEAD -> ')) x = x.slice(8);
    if (x.startsWith('origin/')) out.remotes.push(x.slice(7)); else out.locals.push(x);
  }
  return out;
}
function pillsHtml(c) {
  const { locals, remotes, tags } = refsOf(c);
  const out = [];
  for (const b of locals) {
    const w = wtByBranch[b], both = remotes.includes(b), col = laneColor(b);
    const plus = both ? ' <span class="o">+ origin</span>' : '';
    if (w) {
      out.push(`<span class="pill fill" style="--c:${col}" title="worktree ${esc(w.path)}">⧉ ${esc(b)}${plus}</span>`);
      if (w.claude) out.push(claudeDot(w.claude));
    } else out.push(`<span class="pill" style="--c:${col}">${esc(b)}${plus}</span>`);
  }
  for (const b of remotes) if (!locals.includes(b)) out.push(`<span class="pill remote" style="--c:${laneColor(b)}">origin/${esc(b)}</span>`);
  for (const t of tags) out.push(`<span class="pill tag" style="--c:var(--t-wait)">◆ ${esc(t)}</span>`);
  if (PREDICT_HEADS[c.h] && !S.merging) out.push(`<span class="pill warn" title="${esc(PREDICT_HEADS[c.h])}">⚠ 衝突の予測</span>`);
  return out.join('');
}

/* ── the graph ────────────────────────────────────────────── */
const ROW = 26, LW = 14, PAD = 8;
let G = null;

function buildRows() {
  const hs = heads();
  const top = FAKE.wip.map(w => ({ kind: 'wip', id: 'wip:' + w.wt, t: w.t, wt: w.wt, w, parents: [hs[w.wt]] }));
  if (S.merging) {
    top.push({ kind: 'merging', id: 'merging', t: FAKE.conflict.started, parents: [hs.main, FAKE.conflict.theirs] });
    top.push({ kind: 'commit', id: FAKE.merged.h, t: FAKE.merged.t, c: FAKE.merged, parents: FAKE.merged.p.slice() });
  }
  top.sort((a, b) => b.t - a.t);
  const rows = [...top];
  for (const c of [...FAKE.commits, ...DATA.commits]) rows.push({ kind: 'commit', id: c.h, t: c.t, c, parents: c.p.slice() });
  rows.forEach((r, i) => { r.i = i; });
  return rows;
}

function mergedName(s) {
  let m;
  if ((m = /^Merge pull request #\d+ from [^/]+\/(.+)$/.exec(s))) return m[1];
  if ((m = /^Merge remote-tracking branch '([^']+)'/.exec(s))) return m[1];
  if ((m = /^Merge branch '([^']+)'/.exec(s))) return m[1];
  if ((m = /^Merge (\S+) into/.exec(s))) return m[1];
  return null;
}

/* Names: refs first, the main line, then down first parents (and second
   parents named from the merge's subject). */
function nameRows(rows, byId, main) {
  for (const r of rows) {
    r.name = r.kind === 'wip' ? r.wt : r.kind === 'merging' ? 'main' : null;
    if (r.kind === 'commit') { const { locals, remotes } = refsOf(r.c); r.name = locals[0] ?? remotes[0] ?? null; }
  }
  for (const id of main) byId[id].name = 'main';
  for (const r of rows) {
    if (r.kind === 'commit' && r.parents.length > 1) {
      const q = byId[r.parents[1]];
      if (q && !q.name) q.name = mergedName(r.c.s);
    }
    const q = byId[r.parents[0]];
    if (q && !q.name && r.name) q.name = r.name;
  }
}

/* Folding: every commit that came into main through a merge on main's
   first-parent line belongs to that merge. Worktree heads and refs that are
   not in main yet stay out, with what they need down to main. */
function foldInfo(rows, byId, main) {
  const into = {}, groups = {};
  const order = rows.filter(r => main.has(r.id)).reverse();
  for (const m of order) {
    if (m.kind !== 'commit' || m.parents.length < 2) continue;
    const stack = m.parents.slice(1);
    while (stack.length) {
      const id = stack.pop();
      if (!byId[id] || main.has(id) || into[id]) continue;
      into[id] = m.id;
      (groups[m.id] = groups[m.id] || []).push(id);
      stack.push(...byId[id].parents);
    }
  }
  const keep = new Set();
  const walk = (h, far) => {
    const stack = [h];
    while (stack.length) {
      const id = stack.pop();
      if (!byId[id] || main.has(id) || keep.has(id) || (!far && into[id])) continue;
      keep.add(id);
      stack.push(...byId[id].parents);
    }
  };
  for (const [b, h] of Object.entries(heads())) if (b !== 'main') walk(h, true);
  for (const r of rows) {
    if (r.kind !== 'commit' || into[r.id] || main.has(r.id)) continue;
    const { locals, remotes } = refsOf(r.c);
    if (locals.length || remotes.length) walk(r.id, false);
  }
  return { into, groups, keep };
}

function computeGraph() {
  const all = buildRows();
  const byAll = {};
  all.forEach(r => { byAll[r.id] = r; });
  // Lane 0 belongs to main's first-parent line, always.
  const main = new Set();
  for (let id = S.merging ? 'merging' : heads().main; id && byAll[id] && !main.has(id); id = byAll[id].parents[0]) main.add(id);
  nameRows(all, byAll, main);
  const { into, groups, keep } = foldInfo(all, byAll, main);
  const shown = id => !S.fold || !into[id] || keep.has(id) || S.unfold.has(into[id]);
  const rows = all.filter(r => shown(r.id));
  const byId = {};
  rows.forEach((r, i) => { r.i = i; byId[r.id] = r; });
  for (const r of rows) {
    r.merge = r.parents.length > 1;
    const g = groups[r.id];
    if (g && S.fold) {
      // Count what the fold hides, so an open merge keeps its ▾ chip.
      r.foldN = g.filter(id => !keep.has(id)).length;
      r.open = S.unfold.has(r.id);
      r.foldName = mergedName(r.c.s);
    }
    // A folded merge loses its hidden side; anything else that points into a
    // folded merge goes down first parents to the nearest commit still shown.
    r.parents = r.parents.flatMap((p, k) => {
      if (!byAll[p] || byId[p]) return [p];
      if (k > 0 && main.has(r.id)) return [];
      let q = p;
      while (byAll[q] && !byId[q]) q = byAll[q].parents[0];
      return q ? [q] : [];
    }).filter((p, k, a) => p !== r.id && a.indexOf(p) === k);
  }
  const lanes = [], edges = [];
  const free = () => { let j = 1; while (lanes[j] != null) j++; return j; };
  for (const r of rows) {
    const hits = [];
    lanes.forEach((x, j) => { if (x === r.id) hits.push(j); });
    for (const j of hits) lanes[j] = null;
    r.col = main.has(r.id) ? 0 : hits.length ? hits[0] : free();
    r.parents.forEach((p, k) => {
      if (!byId[p]) { edges.push({ from: r, to: null, via: r.col, first: k === 0 }); return; }
      let v;
      if (k === 0) { v = r.col; lanes[v] = p; } else { v = lanes.indexOf(p); if (v < 0) { v = free(); lanes[v] = p; } }
      edges.push({ from: r, to: p, via: v, first: k === 0 });
    });
    let w = r.col;
    lanes.forEach((x, j) => { if (x != null && j > w) w = j; });
    r.span = w;
  }
  return { rows, byId, edges, into, groups, all: byAll };
}

const X = c => PAD + c * LW + LW / 2;
const Y = i => i * ROW + ROW / 2;
function edgePath(ri, ci, v, rp, cp, cutY) {
  let y = Y(ri);
  const x = X(ci), xv = X(v);
  let d = `M${x} ${y}`;
  if (cutY != null) {
    if (v !== ci) { d += `C${x} ${y + ROW * 0.6} ${xv} ${y + ROW * 0.4} ${xv} ${y + ROW}`; }
    return d + `L${xv} ${cutY}`;
  }
  const yp = Y(rp), xp = X(cp);
  if (rp - ri === 1) return ci === cp ? d + `L${xp} ${yp}` : d + `C${x} ${y + ROW * 0.6} ${xp} ${yp - ROW * 0.6} ${xp} ${yp}`;
  if (v !== ci) { d += `C${x} ${y + ROW * 0.6} ${xv} ${y + ROW * 0.4} ${xv} ${y + ROW}`; y += ROW; }
  if (v !== cp) return d + `L${xv} ${yp - ROW}C${xv} ${yp - ROW * 0.4} ${xp} ${yp - ROW * 0.6} ${xp} ${yp}`;
  return d + `L${xv} ${yp}`;
}

function svgHtml(lim, gw, nShown, more) {
  const parts = [];
  const cutY = more ? lim * ROW + ROW / 2 : null;
  for (const e of G.edges) {
    const r = e.from;
    if (r.i >= lim) continue;
    const p = e.to ? G.byId[e.to] : null;
    const col = e.first ? laneColor(r.name) : laneColor(p ? p.name : r.name);
    const dash = r.kind === 'commit' ? '' : ' stroke-dasharray="3 3"';
    if (!p) {
      parts.push(`<path d="M${X(r.col)} ${Y(r.i)}L${X(r.col)} ${Y(r.i) + ROW * 0.9}" style="stroke:${col}" stroke-opacity=".4" stroke-dasharray="2 3"/>`);
      continue;
    }
    const cut = p.i >= lim;
    parts.push(`<path d="${edgePath(r.i, r.col, e.via, p.i, p.col, cut ? cutY : null)}" style="stroke:${col}"${dash}${cut ? ' stroke-opacity=".45"' : ''}/>`);
  }
  // A folded merge keeps a short stub of the branch that joined it.
  for (let i = 0; i < lim; i++) {
    const r = G.rows[i];
    if (!r.foldN || r.open) continue;
    const x = X(r.col), y = Y(i);
    parts.push(`<path d="M${x + LW * 0.8} ${y + ROW * 0.42}C${x + LW * 0.8} ${y + 2} ${x + 4} ${y} ${x} ${y}" style="stroke:${laneColor(r.foldName)}" stroke-opacity=".8"/>`);
  }
  for (let i = 0; i < lim; i++) {
    const r = G.rows[i], x = X(r.col), y = Y(i), col = laneColor(r.name);
    if (r.kind === 'wip') parts.push(`<circle cx="${x}" cy="${y}" r="4.5" style="fill:var(--t-bg);stroke:${col}" stroke-width="1.6" stroke-dasharray="2.2 1.8"/>`);
    else if (r.kind === 'merging') parts.push(`<circle cx="${x}" cy="${y}" r="5.5" style="fill:var(--t-bg);stroke:var(--t-err)" stroke-width="2"/><circle cx="${x}" cy="${y}" r="2" style="fill:var(--t-err)"/>`);
    else if (r.merge) parts.push(`<circle cx="${x}" cy="${y}" r="5" style="fill:var(--t-bg);stroke:${col}" stroke-width="2"/><circle cx="${x}" cy="${y}" r="2" style="fill:${col}"/>`);
    else parts.push(`<circle cx="${x}" cy="${y}" r="4" style="fill:${col};stroke:var(--t-bg)" stroke-width="1.5"/>`);
  }
  return `<svg class="gsvg" width="${gw}" height="${nShown * ROW}" fill="none" stroke-width="2" stroke-linecap="round" aria-hidden="true">${parts.join('')}</svg>`;
}

/* The branch names inside a merge's group (shown when it is folded). */
function groupRefs(m) {
  const out = [];
  for (const id of G.groups[m] || []) {
    const { locals, remotes } = refsOf(G.all[id].c);
    for (const b of locals) out.push(b);
    for (const b of remotes) if (!locals.includes(b)) out.push('origin/' + b);
  }
  return out;
}
function foldChip(r) {
  const refs = r.open ? [] : groupRefs(r.id);
  const tip = r.open ? 'h で畳む' : `${r.foldN} コミットを畳んでいます（l で開く）` + (refs.length ? `\n中のブランチ: ${refs.join(', ')}` : '');
  return `<span class="fchip${r.open ? ' open' : ''}" data-act="unfold" data-h="${r.id}" style="--c:${laneColor(r.foldName)}" title="${esc(tip)}">${r.open ? '▾' : '▸'} ${r.foldN}</span>`;
}

function rowHtml(r) {
  const cls = ['grow'];
  let sub = '', au = '', hh = '';
  if (r.kind === 'wip') {
    cls.push('wip');
    const w = wtByBranch[r.wt], f = r.w.f;
    sub = `<span class="s">未コミット ${f.length} ファイル（+${sum(f, 1)} −${sum(f, 2)}）</span>`;
    if (w.claude) sub += `<span class="${CL[w.claude.state][2]}" style="font-size:11.5px;flex:none">${CL[w.claude.state][0]} Claude ${CL[w.claude.state][1]}</span>`;
    if (WIP_HITS[r.wt] && !S.merging) sub += `<span class="pill warn wip" title="${esc('まだコミットしていない変更を数えると、' + WIP_HITS[r.wt].join('、') + 'でぶつかる見込み')}">⚠ 衝突の予測</span>`;
    au = '<span class="dim">—</span>';
    hh = 'WIP';
  } else if (r.kind === 'merging') {
    cls.push('wip');
    const left = cfLeft();
    sub = `<span class="pill warn">${left ? `⚠ 衝突 残り ${left}` : '✓ 解けた'}</span><span class="s">マージ中: ${esc(FAKE.conflict.from)} → ${esc(FAKE.conflict.into)}</span>`;
    au = '<span class="dim">—</span>';
    hh = 'MERGE';
  } else {
    const c = r.c;
    if (isUnseen(c.h)) { cls.push('new'); sub += '<span class="nw" title="まだ見ていないコミット"></span>'; }
    sub += pillsHtml(c) + (r.foldN ? foldChip(r) : '') + `<span class="s">${esc(c.s)}</span>`;
    au = c.a === 'github-actions[bot]' ? '<span class="bot" title="github-actions[bot]">⚙ actions</span>' : esc(c.a);
    hh = c.h;
  }
  if (r.i === S.sel) cls.push('sel');
  return `<div class="${cls.join(' ')}" data-i="${r.i}"><div class="c-g"></div><div class="c-sub">${sub}</div><div class="c-au">${au}</div><div class="c-dt">${when(r.t)}</div><div class="c-h">${hh}</div></div>`;
}

function drawGraph() {
  const lim = Math.min(S.limit, G.rows.length);
  const more = G.rows.length - lim;
  let maxLane = 1;
  for (let i = 0; i < lim; i++) maxLane = Math.max(maxLane, G.rows[i].span);
  const gw = PAD * 2 + (maxLane + 1) * LW;
  $('#gcol').style.setProperty('--gw', gw + 'px');
  let html = '';
  for (let i = 0; i < lim; i++) html += rowHtml(G.rows[i]);
  if (more > 0) {
    html += `<div class="grow fold" data-fold="1"><div class="c-g"></div><div class="c-sub"><span class="s">… さらに古い ${more} 件 — 押すと読み込みます（本物は 200 件ずつ）</span></div><div class="c-au"></div><div class="c-dt"></div><div class="c-h"></div></div>`;
  }
  $('#gin').innerHTML = svgHtml(lim, gw, lim + (more > 0 ? 1 : 0), more > 0) + html;
  markFilter();
}

const visibleCount = () => Math.min(S.limit, G.rows.length);

function selectRow(i, opts = {}) {
  const r = G.rows[i];
  if (!r) return;
  if (i >= visibleCount()) { S.limit = G.rows.length; drawGraph(); }
  S.sel = i;
  $$('#gin .grow.sel').forEach(e => e.classList.remove('sel'));
  const el = $(`#gin .grow[data-i="${i}"]`);
  if (el) el.classList.add('sel');
  if (r.kind === 'commit' && isUnseen(r.id)) {
    seen.add(r.id);
    if (el) { el.classList.remove('new'); const dot = el.querySelector('.nw'); if (dot) dot.remove(); }
    renderWt();
  }
  renderDetail();
  if (opts.scroll !== false) ensureVisible(i);
  drawBar();
}

function ensureVisible(i) {
  const list = $('#glist');
  if (!list.clientHeight) return;
  const top = i * ROW;
  if (top < list.scrollTop + ROW) list.scrollTop = Math.max(0, top - ROW * 2);
  else if (top + ROW > list.scrollTop + list.clientHeight - ROW) list.scrollTop = top + ROW * 3 - list.clientHeight;
}

function matchRow(r, q) {
  if (r.kind !== 'commit') return false;
  const c = r.c;
  return c.s.toLowerCase().includes(q) || c.h.startsWith(q) || c.a.toLowerCase().includes(q) || c.r.some(x => x.toLowerCase().includes(q));
}

function onFilter() {
  S.filter = $('#filter').value;
  const q = S.filter.trim().toLowerCase();
  if (q && G.rows.slice(visibleCount()).some(r => matchRow(r, q))) { S.limit = G.rows.length; drawGraph(); return; }
  markFilter();
}

function markFilter() {
  const q = S.filter.trim().toLowerCase();
  let n = 0;
  $$('#gin .grow[data-i]').forEach(el => {
    const hit = !q || matchRow(G.rows[+el.dataset.i], q);
    el.classList.toggle('dimmed', !hit);
    if (q && hit) n++;
  });
  $('#fcount').textContent = q ? `${n} 件` : '';
  drawBar();
}

function nextMatch(dir) {
  const q = S.filter.trim().toLowerCase();
  if (!q) return;
  const n = visibleCount();
  for (let k = 1; k <= n; k++) {
    const i = (S.sel + dir * k + n) % n;
    if (matchRow(G.rows[i], q)) { selectRow(i); return; }
  }
}

/* The thin bar: the graph's only scrollbar, with marks. */
function drawBar() {
  if (S.view !== 'graph') return;
  const list = $('#glist'), bar = $('#gbar');
  const total = list.scrollHeight, view = list.clientHeight, bh = bar.clientHeight;
  if (!bh || !total) return;
  const q = S.filter.trim().toLowerCase();
  const marks = [];
  for (let i = 0; i < visibleCount(); i++) {
    const r = G.rows[i];
    let c = null, t = '';
    if (q && matchRow(r, q)) { c = 'var(--t-fg)'; t = '絞った結果: ' + r.c.s; }
    if (r.kind === 'commit' && r.c.r.some(x => x.startsWith('tag: '))) { c = c || 'var(--t-wait)'; t = t || 'タグ ' + refsOf(r.c).tags.join(' '); }
    if (r.kind === 'commit' && PREDICT_HEADS[r.id] && !S.merging) { c = 'var(--t-err)'; t = '⚠ ' + PREDICT_HEADS[r.id]; }
    const wt = r.kind === 'commit' ? wtOfHead(r.id) : null;
    if (wt && !PREDICT_HEADS[r.id]) { c = laneColor(wt); t = '⧉ ' + wt; }
    if (r.kind === 'wip') { c = 'var(--t-wait)'; t = 'WIP ' + r.wt; }
    if (r.kind === 'merging') { c = 'var(--t-err)'; t = 'マージ中'; }
    if (i === S.sel) { c = 'var(--t-run)'; t = '選んでいる所'; }
    if (c) marks.push(`<div class="mk" data-i="${i}" title="${esc(t)}" style="top:${((i + 0.5) * ROW / total * bh - 1.5).toFixed(1)}px;background:${c}"></div>`);
  }
  const thumb = total > view + 1
    ? `<div class="gthumb" style="top:${(list.scrollTop / total * bh).toFixed(1)}px;height:${Math.max(18, view / total * bh).toFixed(1)}px"></div>` : '';
  bar.innerHTML = marks.join('') + thumb;
}

function moveThumb() {
  const list = $('#glist'), th = $('#gbar .gthumb');
  if (th) th.style.top = (list.scrollTop / list.scrollHeight * $('#gbar').clientHeight).toFixed(1) + 'px';
}

/* ── the detail pane ──────────────────────────────────────── */
function wordTokens(s) { return s.match(/\s+|[A-Za-z0-9_]+|[^\sA-Za-z0-9_]/g) || []; }
function wordDiff(a, b) {
  const A = wordTokens(a), B = wordTokens(b), n = A.length, m = B.length;
  if (!n || !m || n * m > 60000) return [esc(a), esc(b)];
  const dp = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
  for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
  const ma = new Array(n).fill(true), mb = new Array(m).fill(true);
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (A[i] === B[j]) { ma[i] = mb[j] = false; i++; j++; } else if (dp[i + 1][j] >= dp[i][j + 1]) i++; else j++;
  }
  if (dp[0][0] < Math.min(n, m) * 0.3) return [esc(a), esc(b)];
  const wrap = (T, M, tag) => {
    let out = '', open = false;
    T.forEach((t, k) => {
      const on = M[k] && !/^\s+$/.test(t);
      if (on && !open) { out += `<${tag}>`; open = true; }
      if (!on && open) { out += `</${tag}>`; open = false; }
      out += esc(t);
    });
    return open ? out + `</${tag}>` : out;
  };
  return [wrap(A, ma, 'del'), wrap(B, mb, 'ins')];
}

function parseDiff(lines) {
  const out = [];
  let o = 0, n = 0;
  for (const l of lines) {
    if (l === '\x00cut') { out.push({ t: 'cut' }); continue; }
    if (l.startsWith('@@')) {
      const m = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(l);
      if (m) { o = +m[1]; n = +m[2]; }
      out.push({ t: 'hunk', text: l });
      continue;
    }
    const c = l[0];
    if (c === '+') out.push({ t: 'add', n: n++, text: l.slice(1) });
    else if (c === '-') out.push({ t: 'del', o: o++, text: l.slice(1) });
    else if (c === '\\') out.push({ t: 'ctx', text: l });
    else out.push({ t: 'ctx', o: o++, n: n++, text: l.slice(1) });
  }
  for (let i = 0; i < out.length;) {
    if (out[i].t !== 'del') { i++; continue; }
    let j = i;
    while (j < out.length && out[j].t === 'del') j++;
    let k = j;
    while (k < out.length && out[k].t === 'add') k++;
    for (let q = 0; q < Math.min(j - i, k - j); q++) {
      const [a, b] = wordDiff(out[i + q].text, out[j + q].text);
      out[i + q].html = a;
      out[j + q].html = b;
    }
    i = Math.max(k, i + 1);
  }
  for (const r of out) if (r.text != null && r.html == null) r.html = esc(r.text);
  return out;
}

const CUT = '… この先は画面イメージでは省きました';
function unifiedHtml(rows) {
  let h = '<table class="diff"><colgroup><col style="width:38px"><col style="width:38px"><col style="width:12px"><col></colgroup>';
  for (const r of rows) {
    if (r.t === 'hunk') h += `<tr class="hunk"><td class="ln"></td><td class="ln"></td><td class="sg"></td><td class="tx">${esc(r.text)}</td></tr>`;
    else if (r.t === 'cut') h += `<tr class="cut"><td colspan="4">${CUT}</td></tr>`;
    else h += `<tr class="${r.t === 'ctx' ? 'ctx' : r.t}"><td class="ln">${r.o ?? ''}</td><td class="ln">${r.n ?? ''}</td><td class="sg">${r.t === 'add' ? '+' : r.t === 'del' ? '−' : ''}</td><td class="tx">${r.html}</td></tr>`;
  }
  return h + '</table>';
}
function sbsHtml(rows) {
  let h = '<table class="diff sbs"><colgroup><col style="width:38px"><col style="width:12px"><col><col style="width:1px"><col style="width:38px"><col style="width:12px"><col></colgroup>';
  const none = '<td class="ln"></td><td class="sg empty"></td><td class="tx empty"></td>';
  const L = r => r ? `<td class="ln">${r.o}</td><td class="sg del">−</td><td class="tx del">${r.html}</td>` : none;
  const R = r => r ? `<td class="ln">${r.n}</td><td class="sg add">+</td><td class="tx add">${r.html}</td>` : none;
  for (let i = 0; i < rows.length;) {
    const r = rows[i];
    if (r.t === 'hunk') { h += `<tr class="hunk"><td colspan="7" class="tx">${esc(r.text)}</td></tr>`; i++; continue; }
    if (r.t === 'cut') { h += `<tr class="cut"><td colspan="7">${CUT}</td></tr>`; i++; continue; }
    if (r.t === 'ctx') {
      h += `<tr><td class="ln">${r.o ?? ''}</td><td class="sg"></td><td class="tx">${r.html}</td><td class="mid"></td><td class="ln">${r.n ?? ''}</td><td class="sg"></td><td class="tx">${r.html}</td></tr>`;
      i++;
      continue;
    }
    const dels = [], adds = [];
    let j = i;
    while (j < rows.length && rows[j].t === 'del') dels.push(rows[j++]);
    while (j < rows.length && rows[j].t === 'add') adds.push(rows[j++]);
    for (let q = 0; q < Math.max(dels.length, adds.length); q++) h += `<tr>${L(dels[q])}<td class="mid"></td>${R(adds[q])}</tr>`;
    i = j;
  }
  return h + '</table>';
}

function fileStatus(d) {
  if (!d || !d.length) return 'M';
  if (/^@@ -0,0 /.test(d[0])) return 'A';
  if (/ \+0,0 @@/.test(d[0])) return 'D';
  return 'M';
}

function filesHtml(files, diffs, all) {
  const max = Math.max(1, ...files.map(f => Math.max(0, f[1]) + Math.max(0, f[2])));
  const shown = all || files.length <= 12 ? files : files.slice(0, 10);
  let h = '<div class="dfiles">';
  shown.forEach(([p, a, d], k) => {
    const bin = a < 0;
    h += `<div class="df" data-f="${k}"><span class="ic">${fileStatus(diffs && diffs[p])}</span><span class="p" title="${esc(p)}"><bdi>${esc(p)}</bdi></span>`
      + `<span class="num">${bin ? 'バイナリ' : `<span class="a">+${a}</span> <span class="d">−${d}</span>`}</span>`
      + `<span class="bars">${bin ? '' : `<i class="a" style="width:${(a / max * 100).toFixed(1)}%"></i><i class="d" style="width:${(d / max * 100).toFixed(1)}%"></i>`}</span></div>`;
  });
  if (shown.length < files.length) h += `<div class="df" data-act="files-all"><span class="ic">…</span><span class="p dim">ほか ${files.length - shown.length} ファイル — 押すと全部</span><span></span><span></span></div>`;
  return h + '</div>';
}

function diffsHtml(files, diffs) {
  let h = '', first = true;
  files.forEach(([p, a, d], k) => {
    const lines = diffs && diffs[p];
    if (!lines || !lines.length) return;
    h += `<div class="dtitle" data-f="${k}"><b>${esc(p)}</b><span class="num"><span class="a">+${a}</span> <span class="d">−${d}</span></span>`
      + (first ? `<button class="tbtn" data-act="wide">${S.wide ? '縦に戻す <span class="k">Esc</span>' : '横並び <span class="k">Enter</span>'}</button>` : '') + '</div>';
    const rows = parseDiff(lines);
    h += S.wide ? sbsHtml(rows) : unifiedHtml(rows);
    first = false;
  });
  return h;
}

function claudeBox(w, wip) {
  const c = w.claude;
  if (!c) return '';
  const [g, label, cls] = CL[c.state];
  const lead = wip ? (c.state === 'run' ? 'Claude が書いている途中です。' : 'Claude は止まっています。') : `Claude ${label}`;
  return `<div class="wipbox"><b>${lead}</b> <span class="${cls}">${g} ${label}</span>（タブ ${c.tab} · ${esc(c.title)}、${hm(c.since)} から）<br>`
    + `<span class="${c.state === 'wait' ? 'st-wait' : ''}">${esc(c.line)}</span>`
    + `<button class="btn" data-act="focus" data-tab="${c.tab}">タブへ<span class="k">o</span></button>`
    + (wip ? '<button class="btn" data-act="go-changes">変更へ<span class="k">2</span></button>' : '') + '</div>';
}

function renderDetail() {
  const r = G.rows[S.sel], el = $('#detail');
  if (!r) { el.innerHTML = ''; return; }
  if (r.kind === 'merging') { el.innerHTML = mergingDetail(); el.scrollTop = 0; return; }
  let h = '', files, diffs;
  if (r.kind === 'wip') {
    const w = wtByBranch[r.wt];
    h += `<div class="dh"><div class="ttl">${esc(r.wt)} の未コミットの変更</div><div class="meta"><span>worktree <span class="mono">${esc(w.path)}</span></span><span>${hm(r.t)} に最後の書き込み</span></div>`
      + `<div class="wide-hint">ここでは読むだけ。ステージとコミットは「変更」（2）で。</div></div>`;
    h += claudeBox(w, true);
    files = r.w.f;
    diffs = r.w.d;
  } else {
    const c = r.c;
    const ps = c.p.filter(p => p !== '\x00out');
    const out = c.p.includes('\x00out');
    const wt = wtOfHead(c.h);
    const pills = pillsHtml(c);
    h += `<div class="dh"><div class="ttl">${esc(c.s)}</div><div class="meta"><span class="mono">${c.h}</span><span>${c.a === 'github-actions[bot]' ? '⚙ github-actions[bot]' : esc(c.a)}</span><span>${fullTime(c.t)}</span>`
      + (ps.length ? `<span>親 ${ps.map(p => `<span class="plink mono" data-h="${p}">${p}</span>`).join(' ')}</span>` : '')
      + (out ? '<span>親はこの画面イメージの範囲の外</span>' : '') + '</div>'
      + (pills ? `<div class="refs">${pills}</div>` : '')
      + (c.b ? `<div class="body">${esc(c.b)}</div>` : '')
      + `<div class="wide-hint">${S.wide ? 'Esc で樹形図に戻る' : 'Enter で差分を横並びに広げる'}</div></div>`;
    if (wt && wtByBranch[wt].claude) h += claudeBox(wtByBranch[wt], false);
    if (G.groups[c.h]) h += groupHtml(c.h);
    files = c.f;
    diffs = c.d || null;
  }
  h += filesHtml(files, diffs, S.allFiles);
  if (diffs) h += diffsHtml(files, diffs);
  else if (r.c && r.c === FAKE.merged) h += '<div class="dnote">マージのコミットの差分は、次の回（ブランチ全体の差分）で描きます。</div>';
  else h += '<div class="dnote">この画面イメージには、上から 40 件のコミットの差分だけを入れています。</div>';
  el.innerHTML = h;
  el.scrollTop = 0;
}

/* What a merge on main brought in: its group, newest first. */
function groupHtml(m) {
  const g = G.groups[m].slice().sort((a, b) => G.all[b].t - G.all[a].t);
  const state = !S.fold ? '' : S.unfold.has(m) ? '樹形図で開いています（<kbd>h</kbd> で畳む）' : '樹形図では畳んでいます（<kbd>l</kbd> で開く）';
  const shown = g.length > 8 ? g.slice(0, 7) : g;
  let h = `<div class="mgrp"><div class="mg-h">このマージで main に入った ${g.length} コミット<span class="dim">${state}</span></div>`;
  for (const id of shown) {
    const c = G.all[id].c;
    h += `<div class="plink" data-h="${id}"><span class="mono">${id}</span><span class="s">${esc(c.s)}</span><span class="dim">${when(c.t)}</span></div>`;
  }
  if (shown.length < g.length) h += `<div class="dim mg-more">ほか ${g.length - shown.length} コミット</div>`;
  return h + '</div>';
}

function mergingDetail() {
  const C = FAKE.conflict, left = cfLeft();
  let h = `<div class="dh"><div class="ttl">マージ中: ${esc(C.from)} を ${esc(C.into)} に</div><div class="meta"><span>${hm(C.started)} から</span>`
    + `<span>相手 <span class="plink mono" data-h="${C.theirs}">${C.theirs}</span></span><span>base <span class="plink mono" data-h="${C.base}">${C.base}</span></span></div></div>`;
  h += `<div class="wipbox"><b>${left ? `${left} か所がぶつかって止まっています。` : '全部解けました。コミットを待っています。'}</b><button class="btn prime" data-act="go-cf">衝突の解消へ<span class="k">4</span></button></div>`;
  h += '<div class="dfiles">';
  for (const f of C.files) {
    const n = f.blocks.filter(i => !S.cf.blocks[i].done).length;
    h += `<div class="df" data-act="go-cf"><span class="ic ${n ? 'bad' : 'ok'}">${n ? '!' : '✓'}</span><span class="p"><bdi>${esc(f.path)}</bdi></span><span class="num">${n ? n + ' か所' : '解いた'}</span><span></span></div>`;
  }
  for (const [p, why] of C.clean) h += `<div class="df" title="${esc(why)}"><span class="ic ok">✓</span><span class="p"><bdi>${esc(p)}</bdi></span><span class="num dim">自動</span><span></span></div>`;
  return h + '</div>';
}

function setWide(on) {
  S.wide = on;
  $('#v-graph').classList.toggle('wide', on);
  renderDetail();
  requestAnimationFrame(() => { drawBar(); ensureVisible(S.sel); });
}

/* ── the sidebar ──────────────────────────────────────────── */
function findRef(name) {
  for (const r of Object.values(G.all)) {
    if (r.kind !== 'commit') continue;
    const { locals, remotes, tags } = refsOf(r.c);
    if (name.startsWith('origin/') ? remotes.includes(name.slice(7)) : locals.includes(name) || tags.includes(name)) return r.id;
  }
  return null;
}

function renderSide() {
  const hs = heads();
  const si = (attrs, tw, body, n = '', cls = '') => `<div class="si${cls}" ${attrs}><span class="tw">${tw}</span>${body}<span class="n">${n}</span></div>`;
  const sw = b => `<span class="sw" style="background:${laneColor(b)}"></span>`;
  let h = `<div class="sh">worktree<span class="n">${WT.length}</span></div>`;
  for (const w of WT) {
    const st = w.claude ? `<span class="st ${CL[w.claude.state][2]}" title="Claude ${CL[w.claude.state][1]}">${CL[w.claude.state][0]}</span>` : '';
    h += si(`data-go="${hs[w.branch]}" title="${esc(w.path)}"`, '', `${sw(w.branch)}<span class="t">${esc(w.branch)}</span>${st}`, w.ahead ? '↑' + w.ahead : '', w.main ? ' cur' : '');
  }
  const locals = [...new Set(['main', ...WT.map(w => w.branch), ...DATA.locals])];
  const top = locals.filter(b => !b.includes('/'));
  const groups = {};
  for (const b of locals) if (b.includes('/')) { const g = b.split('/')[0]; (groups[g] = groups[g] || []).push(b); }
  h += `<div class="sh">ブランチ<span class="n">${locals.length}</span></div>`;
  for (const b of top) h += si(`data-branch="${esc(b)}"`, '', `${sw(b)}<span class="t">${esc(b)}</span>`);
  for (const g of Object.keys(groups).sort()) {
    const open = !!S.open[g];
    h += si(`data-grp="${esc(g)}"`, open ? '▾' : '▸', `<span class="t">${esc(g)}/</span>`, groups[g].length);
    if (open) for (const b of groups[g].sort()) h += si(`data-branch="${esc(b)}"`, '', `${sw(b)}<span class="t">${esc(b.slice(g.length + 1))}</span>`, '', ' sub');
  }
  const rem = [...new Set([...DATA.remotes.filter(r => r.startsWith('origin/')), 'origin/feat/focus'])].sort();
  h += '<div class="sh">リモート<span class="n">1</span></div>';
  h += si('data-grp="origin"', S.open.origin ? '▾' : '▸', '<span class="t">origin</span>', rem.length);
  if (S.open.origin) for (const r of rem) h += si(`data-branch="${esc(r)}"`, '', `<span class="t">${esc(r.slice(7))}</span>`, '', ' sub');
  h += `<div class="sh">タグ<span class="n">${DATA.tags.length}</span></div>`;
  for (const t of DATA.tags) h += si(`data-branch="${esc(t)}"`, '<span style="color:var(--t-wait)">◆</span>', `<span class="t">${esc(t)}</span>`);
  h += `<div class="sh">stash<span class="n">${FAKE.stash.length}</span></div>`;
  for (const s of FAKE.stash) h += si('data-act="stash-show"', '', `<span class="t" title="${esc(s)}">${esc(s)}</span>`);
  $('#side').innerHTML = h;
}

/* ── the worktree view ────────────────────────────────────── */
function wtState(w) {
  // What the merge scene changes: feat/new-worktree is in main, feat/focus is being merged.
  const s = { ahead: w.ahead, behind: w.behind, note: null };
  if (S.merging && w.branch === 'feat/new-worktree') { s.ahead = 0; s.note = 'main に入った（b4e7d10）'; }
  if (S.merging && w.branch === 'main') { s.note = 'マージ中'; }
  return s;
}
function claudeCell(w) {
  const c = w.claude;
  if (!c) return '<span class="dim">Claude は動いていません</span>';
  const [g, label, cls] = CL[c.state];
  return `<div class="cl"><span class="chip ${cls}">${g} ${label} <span class="dim" style="font-weight:400">${hm(c.since)} から</span></span>`
    + `<span class="line${c.state === 'wait' ? ' ask' : ''}">${esc(c.line)}</span>`
    + `<button class="go" data-act="focus" data-tab="${c.tab}">タブ ${c.tab} · ${esc(c.title)} へ <kbd>o</kbd></button></div>`;
}
function abCell(w) {
  const s = wtState(w);
  const note = s.note || (w.main ? 'origin/main と同じ' : w.pushed ? 'origin に push 済み' : 'まだ push していない');
  return `<div class="ab"><span class="up">↑${s.ahead}</span> <span class="dn">↓${s.behind}</span></div><div class="sub2">${note}</div>`;
}
function wipCell(w) {
  if (S.merging && w.main) return `<span class="cf-bad">マージ中（残り ${cfLeft()}）</span><div class="sub2">衝突の解消（4）</div>`;
  const wip = FAKE.wip.find(x => x.wt === w.branch);
  if (!wip) return '<span class="dim">なし</span>';
  return `${wip.f.length} ファイル <span class="num-a">+${sum(wip.f, 1)}</span> <span class="num-d">−${sum(wip.f, 2)}</span><div class="sub2">${hm(wip.t)} に書き込み</div>`;
}
function unseenCell(w) {
  if (!w.unseen) return '<span class="dim">—</span>';
  const n = unseenOf(w.branch).length;
  if (!n) return '<span class="dim">なし</span>';
  return `<span class="nw"></span>${n} コミット<div class="sub2">${w.unseen.files} ファイル</div>`;
}
function predictCell(w) {
  if (S.merging) {
    if (w.branch === 'feat/focus') return `<span class="cf-bad">⚠ main にマージ中</span><div class="sub2">残り ${cfLeft()} か所</div>`;
    if (w.branch === 'feat/new-worktree') return '<span class="cf-ok">main に入った</span>';
  }
  const pairs = [...Object.entries(FAKE.predict).map(e => [...e, false]), ...Object.entries(PREDICT_WIP).map(e => [...e, true])]
    .filter(([k]) => k.split('|').includes(w.branch));
  if (!pairs.length) return '<span class="cf-ok">なし</span>';
  return pairs.map(([k, v, wip]) => {
    const other = k.split('|').find(x => x !== w.branch);
    const tail = wip ? `${v.map(x => esc(leaf(x[0]))).join('・')}（未コミットの分）` : v.map(x => esc(leaf(x[0]))).join('・');
    return `<div class="cf-one"><span class="cf-bad${wip ? ' wip' : ''}">⚠ ${esc(other)} と ${v.reduce((s, x) => s + x[1], 0)} か所</span><div class="sub2">${tail}</div></div>`;
  }).join('');
}
function actsHtml(w, k) {
  if (w.main) return `<div class="acts"><button data-act="wt-pull" data-k="${k}">pull<span class="k">p</span></button><button data-act="wt-graph" data-k="${k}">樹形図で見る<span class="k">Enter</span></button></div>`;
  return `<div class="acts"><button data-act="wt-diff" data-k="${k}">差分<span class="k">d</span></button><button data-act="wt-merge" data-k="${k}">main を取り込む<span class="k">m</span></button>`
    + `<button data-act="wt-pr" data-k="${k}">PR</button><button class="danger" data-act="wt-rm" data-k="${k}">消す<span class="k">x</span></button></div>`;
}

function renderWt() {
  const short = b => b === 'fix/142-optional-locks' ? 'fix/142' : b === 'feat/new-worktree' ? 'new-worktree' : b === 'feat/focus' ? 'focus' : b;
  let h = `<div class="wt-top"><h2>worktree</h2><span class="dim">tsumugi · ${WT.length} 本 · Claude ${WT.filter(w => w.claude).length} 人</span>`
    + '<button class="tbtn" data-act="refresh">⟳ 読み直す</button><button class="tbtn prime" data-act="new">＋ 新しい作業 <span class="k">N</span></button></div>';
  h += '<table class="wtt"><thead><tr><th>ブランチ・フォルダ</th><th>Claude（tsumugi）</th><th>main から</th><th>未コミット</th><th>見ていない</th><th>衝突の予測</th><th></th></tr></thead><tbody>';
  WT.forEach((w, k) => {
    h += `<tr class="${k === S.wtSel ? 'sel' : ''}" data-k="${k}"><td><div class="wt-name"><span class="sw" style="background:${laneColor(w.branch)}"></span>${esc(w.branch)}</div><div class="wt-path">${esc(w.path)}</div></td>`
      + `<td>${claudeCell(w)}</td><td>${abCell(w)}</td><td>${wipCell(w)}</td><td>${unseenCell(w)}</td><td>${predictCell(w)}</td><td>${actsHtml(w, k)}</td></tr>`;
  });
  h += '</tbody></table>';
  h += '<div class="wt-cards">';
  WT.forEach((w, k) => {
    h += `<div class="card${k === S.wtSel ? ' sel' : ''}" data-k="${k}"><div><div class="wt-name"><span class="sw" style="background:${laneColor(w.branch)}"></span>${esc(w.branch)}</div><div class="wt-path">${esc(w.path)}</div></div>`
      + `<div>${claudeCell(w)}</div>`
      + `<div class="kv"><span>main から</span><span>${abCell(w)}</span></div><div class="kv"><span>未コミット</span><span>${wipCell(w)}</span></div>`
      + `<div class="kv"><span>見ていない</span><span>${unseenCell(w)}</span></div><div class="kv"><span>衝突の予測</span><span>${predictCell(w)}</span></div>`
      + `${actsHtml(w, k)}</div>`;
  });
  h += '</div>';
  // The pairwise prediction and the unseen commits.
  let mx = '<table class="mx"><tr><th></th>' + WT.map(w => `<th>${esc(short(w.branch))}</th>`).join('') + '</tr>';
  WT.forEach((a, i) => {
    mx += `<tr><th class="r">${esc(short(a.branch))}</th>`;
    WT.forEach((b, j) => {
      if (i === j) { mx += '<td class="self">—</td>'; return; }
      const p = predictPair(a.branch, b.branch);
      mx += p && !S.merging
        ? `<td class="hit${p.wip ? ' wip' : ''}${S.mx === p.k ? ' on' : ''}" data-act="mx" data-p="${esc(p.k)}" title="${esc((p.wip ? '未コミットの分を数えると\n' : '') + p.v.map(x => x[0] + ' ' + x[1]).join('\n'))}">${p.v.reduce((s, x) => s + x[1], 0)}</td>`
        : '<td class="dim">·</td>';
    });
    mx += '</tr>';
  });
  mx += '</table>';
  const pair = S.mx && predictPair(...S.mx.split('|'));
  const note = S.merging
    ? 'feat/new-worktree を main に入れたので、予測は「衝突の解消」（4）の実物に変わりました。'
    : pair
      ? `<b>${esc(pair.k.replace('|', ' × '))}</b>: ${pair.v.map(x => `<span class="mono">${esc(x[0])}</span> ${x[1]} か所`).join('、')}。`
        + (pair.wip
          ? 'fix/142 のまだコミットしていない変更（版の繰り上げと CHANGELOG の項目）がぶつかります。コミットすると実線の枠になります。'
          : '先に main に入れた方は何も起きず、後の方を入れるときに解くことになります。')
      : '数を押すと、ぶつかるファイルが出ます。点線の枠は、まだコミットしていない変更を数えてぶつかる組です。';
  const un = Object.keys(UNSEEN).filter(hh => !seen.has(hh));
  const wl = un.length
    ? '<div class="wl">' + un.map(hh => `<div data-h="${hh}"><span class="mono">${hh}</span><span class="s b"><span class="nw"></span>${esc(COMMIT[hh].s)}</span><span class="dim">${esc(short(UNSEEN[hh]))} · ${hm(COMMIT[hh].t)}</span></div>`).join('') + '</div>'
    : '<div class="dim" style="font-size:12px">全部見ました。</div>';
  h += `<div class="wt-low"><div class="box"><h3>ぶつかりの予測（未コミットの変更も数える）</h3>${mx}<div class="mx-note">${note}</div>`
    + '<div class="foot">2 本ずつ <code>git merge-tree --write-tree</code> で合わせてみます（fetch のあと、どれかの HEAD が動いたとき、作業フォルダが書き換わって 2 秒たったとき）。未コミットの変更は、作業フォルダの中身から一時の tree を作って数えます。ブランチも index も動かしません。</div></div>'
    + `<div class="box"><h3>見ていない変更（${un.length}）</h3>${wl}<div class="foot">この表を最後に開いたときから増えたコミット。樹形図で選ぶと消えます。</div></div></div>`;
  h += '<div class="foot">Claude の列は <code>tsumugi ls --json</code> を 2 秒ごと（窓が後ろにあるときは 10 秒）に読みます。tsumugi が動いていなければ列ごと消えます。「タブへ」は <code>tsumugi focus N</code>。</div>';
  $('#v-wt').innerHTML = h;
}

/* ── the conflict view ────────────────────────────────────── */
const TAGS = { ours: '自分', theirs: '相手', both: '両方（自分が先）', both2: '両方（相手が先）', base: 'base', hand: '手で直した', rerere: '前に解いた形', renum: '両方（相手を 18.16 に）' };
const cfFresh = b => b.state === 'rerere' ? { res: b.result.slice(), tag: 'rerere', done: true, checked: false } : { res: null, tag: null, done: false, checked: false };
function cfInit() { S.cf = { cur: 0, showBase: {}, confirm: false, pop: false, to: '7', blocks: CB.map(cfFresh) }; }
const cfLeft = () => S.cf ? S.cf.blocks.filter(b => !b.done).length : 0;
const cfUnchecked = () => S.cf.blocks.filter(b => b.tag === 'rerere' && !b.checked).length;

function cfState(bi) {
  const st = S.cf.blocks[bi];
  if (st.tag === 'rerere' && !st.checked) return `<span class="state st-wait">↺ ${FAKE.conflict.rerere_at} に解いた形を当てました — 確かめてください</span>`;
  if (st.done) return `<span class="state st-done">✓ ${TAGS[st.tag]}</span>`;
  return '<span class="state st-err">● まだ</span>';
}

function cfSideHtml() {
  const C = FAKE.conflict;
  const curFile = C.files.findIndex(f => f.blocks.includes(S.cf.cur));
  let h = `<div class="sh">ぶつかったファイル<span class="n">${C.files.length}</span></div>`;
  C.files.forEach((f, k) => {
    const n = f.blocks.filter(i => !S.cf.blocks[i].done).length;
    h += `<div class="si${k === curFile ? ' on' : ''}" data-cf-file="${k}" title="${esc(f.path)}"><span class="tw"></span><span class="p"><bdi>${esc(f.path)}</bdi></span><span class="left${n ? '' : ' ok'}">${n || '✓'}</span></div>`;
  });
  h += `<div class="sh">自動で合わせた<span class="n">${C.clean.length}</span></div>`;
  for (const [p, why] of C.clean) h += `<div class="si" title="${esc(p)}"><span class="tw"></span><span class="p"><bdi>${esc(p)}</bdi></span><span class="left ok">✓</span></div><div class="why">${esc(why)}</div>`;
  h += '<div class="sh">マージ</div>';
  h += `<div class="why"><b>${esc(C.from)}</b> <span class="mono">${C.theirs}</span> を <b>${esc(C.into)}</b> <span class="mono">${C.head}</span> に。</div>`;
  h += `<div class="why">base <span class="mono">${C.base}</span>（2 本が分かれた所）。${hm(C.started)} から。</div>`;
  h += '<div class="why">「自分」は今いる main、「相手」は入れようとしている feat/focus。</div>';
  return h;
}

function cfCmd() {
  const C = FAKE.conflict;
  const open = CB.map((b, i) => [b, i]).filter(([, i]) => !S.cf.blocks[i].done).map(([b]) => `${b.file} ${b.line} 行目`);
  const msg = `C:\\dev\\tsumugi（main の worktree）で ${C.from} を ${C.into} にマージしていて、${open.length} か所がぶつかっています: ${open.join('、')}。両方の意図を残して解き、git add まで。コミットはしないでください。`;
  return S.cf.to === 'new' ? `tsumugi new C:\\dev\\tsumugi --name "merge ${C.from}" -- claude "${msg}"` : `tsumugi send ${S.cf.to} "${msg}"`;
}

function cfBarHtml() {
  const C = FAKE.conflict, left = cfLeft();
  let h = `<span class="what"><span class="from">${esc(C.from)}</span> を <span class="into">${esc(C.into)}</span> にマージ中</span>`
    + `<span class="cf-count${left ? '' : ' ok'}">${left ? `残り ${left} / ${CB.length} か所` : '全部解けました'}</span><span class="sp"></span>`
    + `<button class="btn" data-act="cf-claude"${left ? '' : ' disabled title="残りがありません"'}>Claude に渡す</button><button class="btn danger" data-act="cf-abort">マージ全体を中止</button>`;
  if (S.cf.confirm) {
    const done = S.cf.blocks.filter(b => b.done && (b.tag !== 'rerere' || b.checked)).length;
    h += `<div class="cf-confirm"><span>マージをやめて、main を <span class="mono">${C.head}</span> に戻します（<span class="mono">git merge --abort</span>）。${done ? `解いた ${done} か所も捨てます。` : ''}</span>`
      + '<button class="btn" data-act="cf-abort-no">続ける</button><button class="btn danger" data-act="cf-abort-yes">中止する</button></div>';
  }
  if (S.cf.pop && left) {
    const opt = (v, l) => `<option value="${v}"${S.cf.to === v ? ' selected' : ''}>${l}</option>`;
    h += `<div class="pop" style="right:14px;top:calc(100% + 6px)"><b>残りの ${left} か所を Claude に頼む</b>`
      + `<div class="row" style="justify-content:flex-start"><span class="dim" style="margin-right:0">送る先</span><select id="cf-to">${opt('7', '7 · tsumugi focus（feat/focus を書いた）')}${opt('6', '6 · new --worktree（feat/new-worktree を書いた）')}${opt('new', '新しいタブで claude を始める')}</select></div>`
      + `<pre id="cf-cmd">${esc(cfCmd())}</pre>`
      + '<div class="row"><span class="dim">解いたあとは、この画面に戻って確かめます</span><button class="btn" data-act="cf-pop-close">やめる</button><button class="btn prime" data-act="cf-send">送る</button></div></div>';
  }
  return h;
}

function cfCtx(lines, from) {
  if (!lines.length) return '';
  return '<div class="ctx">' + lines.map((l, k) => `<div class="l"><span class="n">${from + k}</span><span>${l === '' ? ' ' : esc(l)}</span></div>`).join('') + '</div>';
}

function cfCol(cls, label, ref, lines, start, against) {
  const body = lines.length
    ? lines.map((l, k) => {
      const html = against && against.length && k < against.length && cls !== 'base' ? wordDiff(against[k], l)[1] : esc(l);
      return `<div class="l"><span class="n">${start + k}</span><span>${l === '' ? ' ' : html}</span></div>`;
    }).join('')
    : `<div class="none">${cls === 'base' ? '（なし — 両方が足した）' : '（なし）'}</div>`;
  return `<div class="col ${cls}"><div class="col-h"><b>${label}</b> ${ref}</div><div class="col-b">${body}</div></div>`;
}

function cfBlockHtml(bi) {
  const b = CB[bi], st = S.cf.blocks[bi], C = FAKE.conflict;
  const showBase = S.cf.showBase[bi] ?? !narrow();
  const done = st.done && (st.tag !== 'rerere' || st.checked);
  const why = (w, cls, label) => `<div class="${cls}" title="${esc(w[1])}"><b>${label}</b> ${esc(w[2])} の <span class="mono">${w[0]}</span> ${esc(w[1])} · ${w[3]}</div>`;
  const btn = (act, label, k) => `<button data-act="${act}" data-bi="${bi}">${label}${k ? `<span class="k">${k}</span>` : ''}</button>`;
  let h = `<div class="blk${S.cf.cur === bi ? ' cur' : ''}${done ? ' done' : ''}" data-bi="${bi}">`;
  h += `<div class="blk-h"><span class="no">${bi + 1} / ${CB.length}</span><span>${esc(leaf(b.file))} ${b.line} 行目</span>${cfState(bi)}`
    + `<button class="tbtn" data-act="cf-tbase" data-bi="${bi}">base ${showBase ? 'を隠す' : 'を見る'}</button></div>`;
  h += `<div class="intent">${why(b.why_ours, 'o', '自分')}${why(b.why_theirs, 't', '相手')}</div>`;
  h += `<div class="three${showBase ? '' : ' nobase'}">`
    + cfCol('ours', '自分', `main <span class="mono">${C.head}</span>`, b.ours, b.line, b.base)
    + cfCol('base', 'base', `<span class="mono">${C.base}</span>`, b.base, b.line, null)
    + cfCol('theirs', '相手', `${esc(C.from)} <span class="mono">${C.theirs}</span>`, b.theirs, b.line, b.base) + '</div>';
  h += `<div class="res"><div class="res-h">結果<span class="tag"${st.tag ? '' : ' hidden'}>${st.tag ? TAGS[st.tag] : ''}</span><span class="sp">そのまま書き直せます</span></div>`
    + `<div class="res-b" contenteditable="true" spellcheck="false" data-bi="${bi}" data-ph="下のボタンで選ぶか、ここに書く">${st.res ? esc(st.res.join('\n')) : ''}</div></div>`;
  h += `<div class="hint"><b>ヒント</b><span>${esc(b.hint)}</span>${b.renumber ? `<button class="btn" data-act="cf-renum" data-bi="${bi}">両方を採って相手を 18.16 に</button>` : ''}</div>`;
  h += '<div class="blk-b">';
  if (st.tag === 'rerere' && !st.checked) h += btn('cf-ok', '確かめた', 'Enter') + btn('cf-forget', '忘れる', '');
  h += btn('cf-ours', '自分を採用', 'o') + btn('cf-theirs', '相手を採用', 't') + btn('cf-both', '両方（自分が先）', 'b') + btn('cf-both2', '両方（相手が先）', 'B');
  if (b.base.length) h += btn('cf-base', 'base に戻す', '');
  return h + '</div></div>';
}

function renderCf() {
  if (!S.merging) { $('#v-cf').innerHTML = ''; return; }
  let scroll = $('#cf-scroll');
  const keep = scroll ? scroll.scrollTop : 0;
  if (!scroll) {
    $('#v-cf').innerHTML = '<aside class="cf-side" id="cf-side"></aside><div class="cf-main"><div class="cf-bar" id="cf-bar"></div><div class="cf-scroll" id="cf-scroll"></div></div>';
    scroll = $('#cf-scroll');
  }
  $('#cf-side').innerHTML = cfSideHtml();
  $('#cf-bar').innerHTML = cfBarHtml();
  const C = FAKE.conflict, left = cfLeft();
  let h = '';
  C.files.forEach((f, k) => {
    const n = f.blocks.length;
    h += `<div class="cf-sec" data-file="${k}"><div class="cf-file"><span>${esc(f.path)}</span><span class="dim">${n} か所</span><button class="tbtn" data-act="cf-reset" data-file="${k}">このファイルを最初から</button></div>`;
    f.blocks.forEach((bi, j) => {
      const b = CB[bi];
      if (b.before_skip) h += `<div class="skip">… 同じ ${b.before_skip} 行 …</div>`;
      h += cfCtx(b.before, b.line - b.before.length);
      h += cfBlockHtml(bi);
      const resLen = S.cf.blocks[bi].res ? S.cf.blocks[bi].res.length : b.ours.length;
      h += cfCtx(b.after, b.line + resLen);
      if (j === n - 1 && b.after_skip) h += `<div class="skip">… 同じ ${b.after_skip} 行 …</div>`;
    });
    h += '</div>';
  });
  const unchecked = cfUnchecked();
  h += `<div class="done-bar" id="cf-done"${left ? ' hidden' : ''}><span>${unchecked ? `解けました（↺ 当てた形を ${unchecked} か所、まだ確かめていません）。` : '全部解けました。'}</span>`
    + `<input value="Merge branch '${esc(C.from)}'" spellcheck="false" aria-label="コミットの文"><button class="btn prime" data-act="cf-commit">コミット<span class="k">Ctrl+Enter</span></button></div>`;
  scroll.innerHTML = h;
  scroll.scrollTop = keep;
}

/* After typing in a result: only the counts and that block's head change. */
function cfRefresh(bi) {
  const left = cfLeft();
  $('#cf-side').innerHTML = cfSideHtml();
  const cnt = $('#cf-bar .cf-count');
  cnt.className = 'cf-count' + (left ? '' : ' ok');
  cnt.textContent = left ? `残り ${left} / ${CB.length} か所` : '全部解けました';
  $('#cf-bar [data-act="cf-claude"]').disabled = !left;
  const el = $(`#cf-scroll .blk[data-bi="${bi}"]`), st = S.cf.blocks[bi];
  el.classList.toggle('done', st.done);
  el.querySelector('.state').outerHTML = cfState(bi);
  const tag = el.querySelector('.res-h .tag');
  tag.hidden = !st.tag;
  tag.textContent = st.tag ? TAGS[st.tag] : '';
  $('#cf-done').hidden = left > 0;
  renderChrome();
}

/* The next block still open; once none is, the next rerere guess not yet checked. */
function cfNextOpen(from) {
  const tests = [st => !st.done, st => st.tag === 'rerere' && !st.checked];
  for (const t of tests) for (let k = 1; k <= CB.length; k++) { const i = (from + k) % CB.length; if (t(S.cf.blocks[i])) return i; }
  return null;
}
function cfSet(bi, tag, fromKey) {
  const b = CB[bi], st = S.cf.blocks[bi];
  const lines = {
    ours: b.ours, theirs: b.theirs, base: b.base, both: [...b.ours, ...b.theirs], both2: [...b.theirs, ...b.ours],
    renum: b.renumber ? [...b.ours, ...b.theirs.map(l => l.replace(b.renumber[0], b.renumber[1]))] : null,
  }[tag];
  Object.assign(st, { res: lines.slice(), tag, done: true, checked: true });
  S.cf.cur = bi;
  if (fromKey) { const n = cfNextOpen(bi); if (n != null) S.cf.cur = n; }
  afterCf(fromKey);
}
function afterCf(scroll) {
  renderCf();
  renderChrome();
  if (scroll) cfScrollTo(S.cf.cur);
}
function cfScrollTo(bi) {
  const sc = $('#cf-scroll'), el = $(`.blk[data-bi="${bi}"]`, sc);
  if (el) sc.scrollTop = Math.max(0, el.offsetTop - 70);
}
function cfMove(dir) {
  S.cf.cur = (S.cf.cur + dir + CB.length) % CB.length;
  $$('#cf-scroll .blk').forEach(e => e.classList.toggle('cur', +e.dataset.bi === S.cf.cur));
  $('#cf-side').innerHTML = cfSideHtml();
  cfScrollTo(S.cf.cur);
}

/* ── chrome: tabs, title-bar chip, status bar ─────────────── */
const SBKEYS = {
  graph: [['j k', '動かす'], ['Enter', '横並び'], ['/', '絞る'], ['z', 'PR を畳む'], ['o', 'タブへ'], ['N', '新しい作業'], ['f', 'fetch'], ['?', 'キー']],
  changes: [['1', '樹形図'], ['3', 'worktree'], ['?', 'キー']],
  wt: [['j k', '選ぶ'], ['o', 'タブへ'], ['Enter', '樹形図で見る'], ['m', 'main を取り込む'], ['x', '消す'], ['N', '新しい作業'], ['?', 'キー']],
  cf: [[']  [', '次・前の箇所'], ['o', '自分'], ['t', '相手'], ['b B', '両方'], ['e', '結果を直す'], ['?', 'キー']],
};
function renderChrome() {
  const views = [['graph', '1', '樹形図'], ['changes', '2', '変更'], ['wt', '3', 'worktree']];
  if (S.merging) views.push(['cf', '4', '衝突']);
  const wipFiles = FAKE.wip.reduce((s, w) => s + w.f.length, 0);
  $('#tabs').innerHTML = views.map(([v, k, l]) => `<button class="tab${S.view === v ? ' on' : ''}" data-view="${v}"><span class="k">${k}</span>${l}`
    + (v === 'changes' ? ` <span class="dim" style="font-size:11px">${wipFiles}</span>` : '')
    + (v === 'cf' ? ` <span class="alarm">● ${cfLeft() || '✓'}</span>` : '') + '</button>').join('');
  $('#tb-chip').innerHTML = `<b>tsumugi</b><span class="dim">⎇ main · worktree ${WT.length}</span>${S.merging ? '<span style="color:var(--t-err)">· マージ中</span>' : ''}`;
  $('#sb-keys').innerHTML = SBKEYS[S.view].map(([k, l]) => `<span><b>${k}</b>${l}</span>`).join('');
  const n = st => WT.filter(w => w.claude && w.claude.state === st).length;
  $('#sb-right').innerHTML = '<span>⟳ fetch 3 分前</span>'
    + `<span>Claude <span class="run">● ${n('run')}</span> <span class="wait">◐ ${n('wait')}</span> <span class="st-done">✓ ${n('done')}</span></span>`
    + `<span>2026/10/15 (Thu) ${hm(S.merging ? FAKE.now_merge : FAKE.now)}</span>`;
  $('#tb-logo').innerHTML = logoSvg(LOGO);
}

const VIEWS = { graph: '#v-graph', changes: '#v-changes', wt: '#v-wt', cf: '#v-cf' };
function setView(v) {
  if (v === 'cf' && !S.merging) { toast('衝突はありません。上の「場面」を「衝突の解消」にすると、マージ中の窓になります。'); return; }
  S.view = v;
  for (const [k, sel] of Object.entries(VIEWS)) $(sel).hidden = k !== v;
  renderChrome();
  if (v === 'graph') requestAnimationFrame(() => { drawBar(); ensureVisible(S.sel); });
}

/* ── overlays and toasts ──────────────────────────────────── */
function toast(html, ms = 3200) {
  const t = $('#toast');
  t.innerHTML = html;
  t.hidden = false;
  clearTimeout(toast.tm);
  toast.tm = setTimeout(() => { t.hidden = true; }, ms);
}
const MOCK = '画面イメージなので、押しても何も起きません。';

function openOv(html) {
  const ov = $('#ov');
  ov.innerHTML = html;
  ov.hidden = false;
  const f = ov.querySelector('input, textarea');
  (f || ov.querySelector('.dlg')).focus({ preventScroll: true });
}
function closeOv() {
  const ov = $('#ov');
  ov.hidden = true;
  ov.innerHTML = '';
  $('#win').focus({ preventScroll: true });
}

function keysDlg() {
  const sec = (h, list) => `<h4>${h}</h4>` + list.map(([k, l]) => `<div><span>${l}</span><span>${k.split(' ').map(x => `<kbd>${esc(x)}</kbd>`).join(' ')}</span></div>`).join('');
  openOv('<div class="dlg" tabindex="-1" style="width:640px"><div class="dlg-h">キー<span class="dim">lazygit に寄せた既定。設定（,）で変えられます</span></div><div class="dlg-b"><div class="keys">'
    + sec('どこでも', [['1 2 3', '画面を切り替える'], ['N', '新しい作業（worktree + Claude）'], ['f', 'fetch'], ['p', 'pull'], ['P', 'push'], [',', '設定'], ['Ctrl+Shift+P', 'コマンドを探す'], ['?', 'この一覧']])
    + sec('樹形図', [['j k', '上下に動かす'], ['Enter', '差分を横並びに広げる'], ['Esc', '戻る・絞りを消す'], ['/', '絞る（Enter で次）'], ['z', 'PR を畳む（main の first-parent だけ）・広げる'], ['l h', '選んだマージを開く・畳む'], ['b', 'ブランチを作る・切り替える'], ['s', 'stash'], ['o', 'その worktree の Claude のタブへ']])
    + sec('worktree', [['j k', '行を選ぶ'], ['Enter', '樹形図で見る'], ['o', 'Claude のタブへ（tsumugi focus）'], ['d', 'ブランチ全体の差分'], ['m', 'main を取り込む'], ['x', '消す']])
    + sec('衝突の解消（マージ中だけ 4）', [['] [', '次・前の箇所'], ['o', '自分を採用'], ['t', '相手を採用'], ['b', '両方（自分が先）'], ['B', '両方（相手が先）'], ['e', '結果を直す'], ['Enter', '当てた形を確かめた'], ['Ctrl+Enter', 'マージをコミット']])
    + '</div></div><div class="dlg-f"><button class="btn" data-act="ov-close">閉じる<span class="k">Esc</span></button></div></div>');
}

const NW = { n: 1 };
function newDlg() {
  NW.n = 1;
  const r = G.rows[S.sel];
  const selOpt = r && r.kind === 'commit' ? `<option value="${r.id}">選んでいるコミット ${r.id}</option>` : '';
  openOv('<div class="dlg" tabindex="-1"><div class="dlg-h">新しい作業<span class="dim">worktree を作り、そこで Claude を始める</span></div><div class="dlg-b">'
    + '<div class="fld"><label for="nw-br">ブランチ</label><div><input id="nw-br" class="mono" value="tsumugi/1015-1012" spellcheck="false" autocomplete="off"><div class="help">フォルダは <span class="mono" id="nw-dir"></span>（リポジトリの横）。作業の名前に変えてかまいません。</div></div></div>'
    + `<div class="fld"><label for="nw-from">元</label><div><select id="nw-from"><option value="origin/main">origin/main（3 分前に fetch）</option><option value="main">main（この PC の main）</option>${selOpt}</select></div></div>`
    + '<div class="fld"><label for="nw-task">指示</label><div><textarea id="nw-task" placeholder="Claude への最初の指示（空なら claude を始めるだけ）"></textarea></div></div>'
    + '<div class="fld"><label>本数</label><div><div class="stepper"><button data-act="nw-dec" aria-label="減らす">−</button><b id="nw-n">1</b><button data-act="nw-inc" aria-label="増やす">＋</button></div><div class="help">2 本以上なら同じ指示で並べて始めます（ブランチは -1、-2 …）。1〜8 本。</div></div></div>'
    + '<div class="fld"><label>回すもの</label><div class="cmd" id="nw-cmd"></div></div>'
    + '</div><div class="dlg-f"><button class="btn" data-act="ov-close">やめる<span class="k">Esc</span></button><button class="btn prime" data-act="nw-go">始める<span class="k">Ctrl+Enter</span></button></div></div>');
  nwPreview();
}
function nwPreview() {
  const br = $('#nw-br').value.trim() || 'tsumugi/1015-1012', from = $('#nw-from').value, task = $('#nw-task').value.trim();
  $('#nw-n').textContent = NW.n;
  $('#nw-dir').textContent = 'C:\\dev\\tsumugi-' + br.replace(/[\\/]/g, '-');
  const one = b => `tsumugi new C:\\dev\\tsumugi --worktree ${b} --from ${from} -- claude${task ? ` "${task.replace(/"/g, '\\"')}"` : ''}`;
  const lines = NW.n === 1 ? [one(br)] : Array.from({ length: NW.n }, (_, k) => one(`${br}-${k + 1}`));
  $('#nw-cmd').innerHTML = '<span class="c"># tsumugi が worktree を作り、そこにタブを開いて Claude を始める</span>\n' + lines.map(esc).join('\n');
}

function rmDlg(k) {
  const w = WT[k], wip = FAKE.wip.find(x => x.wt === w.branch);
  let items = '';
  if (w.claude) items += `<li class="warn">Claude のタブ ${w.claude.tab}（${esc(w.claude.title)}）が${CL[w.claude.state][1]}です。先に閉じます（<code>tsumugi close ${w.claude.tab}</code>）。</li>`;
  items += wip ? `<li class="warn">コミットしていない変更が ${wip.f.length} ファイルあります。消すと戻せません。</li>` : '<li>コミットしていない変更はありません。</li>';
  if (w.pushed) items += '<li>origin に push 済みです。</li>';
  else if (w.ahead) items += `<li class="warn">push していないコミットが ${w.ahead} 個あります（ブランチには残ります）。</li>`;
  openOv(`<div class="dlg" tabindex="-1"><div class="dlg-h">worktree を消す<span class="dim">${esc(w.branch)}</span></div><div class="dlg-b">`
    + `<p>フォルダ <code>${esc(w.path)}</code> を消します。ブランチ <code>${esc(w.branch)}</code> は残します。</p><ul>${items}</ul></div>`
    + `<div class="dlg-f"><button class="btn" data-act="ov-close">やめる<span class="k">Esc</span></button><button class="btn danger" data-act="rm-go" data-k="${k}">消す</button></div></div>`);
}

/* ── actions ──────────────────────────────────────────────── */
function goCommit(h) {
  if (!G.byId[h] && G.into[h]) { S.unfold.add(G.into[h]); regraph(); }
  const r = G.byId[h];
  if (!r) { toast(`<code>${esc(h)}</code> は、この画面イメージの 130 件より古いコミットです。`); return; }
  if (S.view !== 'graph') setView('graph');
  selectRow(r.i);
}

/* Recompute the graph after folding changed, keeping the selection (or the
   merge it went into). */
function regraph() {
  const id = G.rows[S.sel] && G.rows[S.sel].id;
  G = computeGraph();
  let i = G.rows.findIndex(r => r.id === id);
  if (i < 0 && G.into[id]) i = G.rows.findIndex(r => r.id === G.into[id]);
  S.sel = Math.max(0, i);
  if (S.sel >= S.limit) S.limit = G.rows.length;
  drawGraph();
  renderDetail();
  syncFold();
  requestAnimationFrame(() => ensureVisible(S.sel));
}
function syncFold() {
  const b = $('#fold-btn');
  if (b) b.setAttribute('aria-pressed', String(S.fold));
}
function setFold(on) {
  S.fold = on;
  store.set('fold', on ? '1' : '0');
  S.unfold.clear();
  regraph();
  const n = Object.keys(G.groups).length, c = Object.keys(G.into).length;
  toast(on ? `main に入った ${n} 本のマージを畳みました（${c} コミット）。worktree と、まだ main に入っていないブランチは出たままです。` : 'マージを全部広げました。');
}
/* l opens the selected folded merge; h folds it again, also from inside it. */
function foldSel(open) {
  const r = G.rows[S.sel];
  if (!r || !S.fold) { toast('z で PR を畳むと使えます。'); return; }
  const m = r.foldN != null ? r.id : G.into[r.id];
  if (!m || !G.groups[m]) { toast('選んでいる行は、main に入ったマージではありません。'); return; }
  if (open) {
    if (m !== r.id || r.open) return;
    S.unfold.add(m);
  } else {
    if (!S.unfold.has(m)) return;
    S.unfold.delete(m);
  }
  regraph();
}

function act(name, el) {
  const k = el.dataset.k != null ? +el.dataset.k : S.wtSel;
  const bi = el.dataset.bi != null ? +el.dataset.bi : S.cf && S.cf.cur;
  switch (name) {
    case 'fetch': toast('⟳ fetch しました。新しいコミットはありません（画面イメージ）。'); break;
    case 'pull': case 'wt-pull':
      if (S.merging && (name === 'pull' || WT[k].main)) { toast('↓ pull: ' + LOCKED); break; }
      toast('↓ pull: 既定は <code>--ff-only</code>。進められないときは理由を出して止めます。' + MOCK); break;
    case 'push': toast('↑ push: 先に分かれていないかを確かめ、force は別のキーにします。' + MOCK); break;
    case 'branch':
      toast(S.merging ? 'ブランチは作れます。切り替え（checkout）は、マージ中なのでこの worktree（main）では止めています。' : 'ブランチの作成・切り替えは小さなパレットで出します。' + MOCK); break;
    case 'stash':
      if (S.merging) { toast('stash: ' + LOCKED); break; }
      toast('stash の中身は次の回（変更の画面）で描きます。'); break;
    case 'stash-show': toast('stash の中身は次の回（変更の画面）で描きます。'); break;
    case 'pal': toast('コマンドを探す: ito のコマンドパレット（tsumugi・kura と同じ）。'); break;
    case 'new': newDlg(); break;
    case 'wide': setWide(!S.wide); break;
    case 'fold': setFold(!S.fold); break;
    case 'unfold': {
      const m = el.dataset.h;
      if (S.unfold.has(m)) S.unfold.delete(m); else S.unfold.add(m);
      regraph();
      break;
    }
    case 'files-all': S.allFiles = true; renderDetail(); S.allFiles = false; break;
    case 'focus': toast(`<code>tsumugi focus ${el.dataset.tab}</code> を回しました。tsumugi の窓が前に出て、タブ ${el.dataset.tab} が選ばれます。`); break;
    case 'go-cf': setView('cf'); break;
    case 'go-changes': setView('changes'); break;
    case 'refresh': toast('⟳ worktree と Claude の様子を読み直しました。'); break;
    case 'mx': S.mx = S.mx === el.dataset.p ? false : el.dataset.p; renderWt(); break;
    case 'wt-graph': setView('graph'); goCommit(heads()[WT[k].branch]); break;
    case 'wt-diff': toast(`ブランチ全体の差分（<code>main...${esc(WT[k].branch)}</code>）と「見た印」は次の回で描きます。`); break;
    case 'wt-merge': toast(`main を ${esc(WT[k].branch)} に取り込みます。既定は merge、設定で rebase。rebase の画面は次の回。`); break;
    case 'wt-pr': toast(`<code>gh pr create --head ${esc(WT[k].branch)}</code> を回し、できた PR の番号をこの行に出します。`); break;
    case 'wt-rm': rmDlg(k); break;
    case 'rm-go': closeOv(); toast(`<code>tsumugi close</code> → <code>git worktree remove</code> の順に回します。${MOCK}`); break;
    case 'ov-close': closeOv(); break;
    case 'nw-dec': NW.n = Math.max(1, NW.n - 1); nwPreview(); break;
    case 'nw-inc': NW.n = Math.min(8, NW.n + 1); nwPreview(); break;
    case 'nw-go': closeOv(); toast(`始めると worktree の表に ${NW.n} 行増え、tsumugi にタブが開きます。${MOCK}`); break;
    case 'cf-ours': cfSet(bi, 'ours'); break;
    case 'cf-theirs': cfSet(bi, 'theirs'); break;
    case 'cf-both': cfSet(bi, 'both'); break;
    case 'cf-both2': cfSet(bi, 'both2'); break;
    case 'cf-base': cfSet(bi, 'base'); break;
    case 'cf-renum': cfSet(bi, 'renum'); break;
    case 'cf-ok': S.cf.blocks[bi].checked = true; S.cf.cur = bi; afterCf(); break;
    case 'cf-forget': Object.assign(S.cf.blocks[bi], { res: null, tag: null, done: false, checked: false }); S.cf.cur = bi; afterCf(); break;
    case 'cf-tbase': S.cf.showBase[bi] = !(S.cf.showBase[bi] ?? !narrow()); renderCf(); break;
    case 'cf-reset': {
      const own = FAKE.conflict.files[+el.dataset.file].blocks;
      for (const i of own) S.cf.blocks[i] = cfFresh(CB[i]);
      S.cf.cur = own[0];
      afterCf(true);
      break;
    }
    case 'cf-abort': S.cf.confirm = !S.cf.confirm; S.cf.pop = false; renderCf(); break;
    case 'cf-abort-no': S.cf.confirm = false; renderCf(); break;
    case 'cf-abort-yes': S.cf.confirm = false; renderCf(); toast(`<code>git merge --abort</code> を回して樹形図に戻ります。${MOCK}`); break;
    case 'cf-claude': S.cf.pop = !S.cf.pop; S.cf.confirm = false; renderCf(); break;
    case 'cf-pop-close': S.cf.pop = false; renderCf(); break;
    case 'cf-send': S.cf.pop = false; renderCf(); toast(`送りました。Claude が解いて <code>git add</code> したら、ここの「残り」が減ります。${MOCK}`); break;
    case 'cf-commit': toast(`マージのコミットを作って樹形図に戻ります。${MOCK}`); break;
    default: toast(MOCK);
  }
}

function onClick(e) {
  const t = e.target;
  if (t === $('#ov')) { closeOv(); return; }
  const tab = t.closest('.tab');
  if (tab) { setView(tab.dataset.view); return; }
  const a = t.closest('[data-act]');
  if (a && !a.classList.contains('si') || a && a.dataset.act === 'stash-show') { act(a.dataset.act, a); return; }
  const pl = t.closest('.plink');
  if (pl) { goCommit(pl.dataset.h); return; }
  const row = t.closest('.grow');
  if (row) {
    if (row.dataset.fold) { S.limit = G.rows.length; drawGraph(); drawBar(); return; }
    selectRow(+row.dataset.i, { scroll: false });
    return;
  }
  const df = t.closest('.df[data-f]');
  if (df) {
    $$('#detail .df.on').forEach(x => x.classList.remove('on'));
    df.classList.add('on');
    const d = $(`#detail .dtitle[data-f="${df.dataset.f}"]`);
    if (d) $('#detail').scrollTop = d.offsetTop - 4;
    else toast('このファイルの差分は画面イメージに入れていません。');
    return;
  }
  const si = t.closest('.side .si');
  if (si) {
    if (si.dataset.grp) { S.open[si.dataset.grp] = !S.open[si.dataset.grp]; renderSide(); return; }
    if (si.dataset.go) { goCommit(si.dataset.go); return; }
    if (si.dataset.branch) {
      const h = findRef(si.dataset.branch);
      if (h) goCommit(h); else toast(`${esc(si.dataset.branch)} の先頭は、この画面イメージの 130 件より古いところにあります。`);
    }
    return;
  }
  const cs = t.closest('.cf-side .si[data-cf-file]');
  if (cs) {
    const f = FAKE.conflict.files[+cs.dataset.cfFile];
    S.cf.cur = f.blocks[0];
    cfMove(0);
    return;
  }
  const wr = t.closest('.wtt tr[data-k], .card[data-k]');
  if (wr) {
    S.wtSel = +wr.dataset.k;
    $$('#v-wt tr[data-k], #v-wt .card[data-k]').forEach(x => x.classList.toggle('sel', +x.dataset.k === S.wtSel));
    return;
  }
  const wl = t.closest('.wl > div[data-h]');
  if (wl) { goCommit(wl.dataset.h); return; }
  const blk = t.closest('.blk');
  if (blk && S.cf.cur !== +blk.dataset.bi) {
    S.cf.cur = +blk.dataset.bi;
    $$('#cf-scroll .blk').forEach(x => x.classList.toggle('cur', x === blk));
    $('#cf-side').innerHTML = cfSideHtml();
  }
}

function onKey(e) {
  const win = $('#win');
  const ae = document.activeElement;
  if (ae && ae !== document.body && !win.contains(ae)) return;
  const t = e.target;
  const typing = t.matches && t.matches('input, textarea, select, [contenteditable="true"]');
  const k = e.key;
  const stop = () => { e.preventDefault(); e.stopPropagation(); };
  if (!$('#ov').hidden) {
    if (k === 'Escape') { closeOv(); stop(); } else if (k === 'Enter' && (e.ctrlKey || e.metaKey) && $('#nw-cmd')) { act('nw-go', $('#ov')); stop(); }
    return;
  }
  if (typing) {
    if (k === 'Escape') {
      if (t.id === 'filter') { t.value = ''; onFilter(); }
      win.focus({ preventScroll: true });
      stop();
    } else if (t.id === 'filter' && k === 'Enter') { nextMatch(e.shiftKey ? -1 : 1); stop(); }
    else if (k === 'Enter' && (e.ctrlKey || e.metaKey) && S.view === 'cf' && !cfLeft()) { act('cf-commit', t); stop(); }
    return;
  }
  if (e.ctrlKey && e.shiftKey && (k === 'P' || k === 'p')) { act('pal', win); stop(); return; }
  if (k === 'Enter' && (e.ctrlKey || e.metaKey) && S.view === 'cf') { if (!cfLeft()) act('cf-commit', win); stop(); return; }
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  if (k === '?' || k === 'F1') { keysDlg(); stop(); return; }
  if (k === ',') { toast('設定は ito-prefs の共通の画面（tsumugi・kura と同じ形）を開きます。'); stop(); return; }
  if (k === '1') { setView('graph'); stop(); return; }
  if (k === '2') { setView('changes'); stop(); return; }
  if (k === '3') { setView('wt'); stop(); return; }
  if (k === '4') { setView('cf'); stop(); return; }
  if (k === 'N') { newDlg(); stop(); return; }
  if (S.view === 'graph' || S.view === 'wt') {
    if (k === 'f') { act('fetch', win); stop(); return; }
    if (k === 'p') { act('pull', win); stop(); return; }
    if (k === 'P') { act('push', win); stop(); return; }
  }
  if (S.view === 'graph') {
    const r = G.rows[S.sel];
    const step = d => {
      let i = S.sel + d;
      while (i >= 0 && i < visibleCount() && S.filter && $(`#gin .grow[data-i="${i}"]`)?.classList.contains('dimmed')) i += d;
      if (i >= 0 && i < visibleCount()) selectRow(i);
    };
    if (k === 'j' || k === 'ArrowDown') { step(1); stop(); return; }
    if (k === 'k' || k === 'ArrowUp') { step(-1); stop(); return; }
    if (k === 'Home' || k === 'g') { selectRow(0); stop(); return; }
    if (k === 'End' || k === 'G') { selectRow(visibleCount() - 1); stop(); return; }
    if (k === 'Enter') { setWide(true); stop(); return; }
    if (k === 'Escape') { if (S.wide) setWide(false); else if (S.filter) { $('#filter').value = ''; onFilter(); } stop(); return; }
    if (k === '/') { $('#filter').focus(); stop(); return; }
    if (k === 'b') { act('branch', win); stop(); return; }
    if (k === 's') { act('stash', win); stop(); return; }
    if (k === 'z') { setFold(!S.fold); stop(); return; }
    if (k === 'l' || k === 'ArrowRight') { foldSel(true); stop(); return; }
    if (k === 'h' || k === 'ArrowLeft') { foldSel(false); stop(); return; }
    if (k === 'o') {
      const b = r.kind === 'wip' ? r.wt : r.kind === 'commit' ? wtOfHead(r.id) : null;
      const w = b && wtByBranch[b];
      if (w && w.claude) act('focus', { dataset: { tab: String(w.claude.tab) } });
      else toast('選んでいる行には Claude のタブがありません。');
      stop();
      return;
    }
  }
  if (S.view === 'wt') {
    const w = WT[S.wtSel];
    const sel = i => { S.wtSel = (i + WT.length) % WT.length; renderWt(); };
    if (k === 'j' || k === 'ArrowDown') { sel(S.wtSel + 1); stop(); return; }
    if (k === 'k' || k === 'ArrowUp') { sel(S.wtSel - 1); stop(); return; }
    if (k === 'Enter') { act('wt-graph', { dataset: {} }); stop(); return; }
    if (k === 'o') { if (w.claude) act('focus', { dataset: { tab: String(w.claude.tab) } }); else toast('この worktree では Claude が動いていません。'); stop(); return; }
    if (k === 'd' && !w.main) { act('wt-diff', { dataset: {} }); stop(); return; }
    if (k === 'm' && !w.main) { act('wt-merge', { dataset: {} }); stop(); return; }
    if (k === 'x' && !w.main) { rmDlg(S.wtSel); stop(); return; }
  }
  if (S.view === 'cf') {
    if (k === ']') { cfMove(1); stop(); return; }
    if (k === '[') { cfMove(-1); stop(); return; }
    const map = { o: 'ours', t: 'theirs', b: 'both', B: 'both2' };
    if (map[k]) { cfSet(S.cf.cur, map[k], true); stop(); return; }
    if (k === 'Enter') {
      const st = S.cf.blocks[S.cf.cur];
      if (st.tag === 'rerere' && !st.checked) { st.checked = true; afterCf(true); }
      stop();
      return;
    }
    if (k === 'e') {
      const ed = $(`#cf-scroll .res-b[data-bi="${S.cf.cur}"]`);
      ed.focus({ preventScroll: true });
      const range = document.createRange();
      range.selectNodeContents(ed);
      range.collapse(false);
      const s = getSelection();
      s.removeAllRanges();
      s.addRange(range);
      stop();
    }
  }
}

/* ── the page around the window ───────────────────────────── */
function layout() {
  const win = $('#win'), stage = $('#stage'), frame = $('#frame');
  win.classList.toggle('narrow', narrow());
  const W = narrow() ? 960 : 1280, H = 800;
  frame.style.width = W + 'px';
  if (S.fit === 'fit') {
    stage.classList.remove('real');
    S.k = Math.min(1, stage.clientWidth / W);
    frame.style.transform = S.k < 1 ? `scale(${S.k})` : '';
    stage.style.height = Math.ceil(H * S.k) + 'px';
  } else {
    stage.classList.add('real');
    S.k = 1;
    frame.style.transform = '';
    stage.style.height = '';
  }
}

function syncSeg() {
  $$('#controls .seg').forEach(seg => {
    const v = S[seg.dataset.ctl];
    $$('button', seg).forEach(b => b.setAttribute('aria-pressed', String(b.dataset.v === v)));
  });
}

function renderLogos() {
  const tile = cls => `<div class="tile ${cls}"><div class="big">${logoSvg(LOGO)}</div><div class="sm">${logoSvg(LOGO, 's32')}${logoSvg(LOGO, 's18')}${logoSvg(LOGO, 's16')}</div><div class="lock">${logoSvg(LOGO)}<span>michi</span></div></div>`;
  const icon = `<div class="tile ic"><div class="big">${iconSvg('i72')}</div><div class="sm">${iconSvg('s32')}${iconSvg('s18')}${iconSvg('s16')}</div><div class="lock"><span>アプリのアイコン</span></div></div>`;
  const others = Object.keys(LOGOS).filter(k => k !== LOGO)
    .map(k => `<div class="lother">${logoSvg(k)}<div><b>${LOGOS[k].name}</b><span>${LOGOS[k].note}</span></div></div>`).join('');
  $('#logos').innerHTML = `<h2>ロゴ: ${LOGOS[LOGO].name}（2026-10-15 に決定）</h2>`
    + `<p>${LOGOS[LOGO].note} 字は michi だけです。窓の中ではテーマの run（シアン）と wait（黄）で描き、下の窓のタイトルバーもテーマを変えると追いかけます。アプリのアイコン（右）は tsumugi Dark の色で固定で、リポの <code>assets/icon.svg</code> そのものです。小さい 3 つは 32・18・16 px（タスクバーとタブ）。</p>`
    + `<div class="lopt chosen"><div class="tiles">${tile('dk')}${tile('lt')}${icon}</div></div>`
    + `<div class="wordmark"><div class="wm dk">${logoSvg(LOGO)}<span>michi</span></div><div class="wm lt">${logoSvg(LOGO)}<span>michi</span></div></div>`
    + `<h3 class="lh">選ばなかった案</h3><div class="lothers">${others}</div>`;
}

const NOTES = {
  graph: [
    ['この画面で見るもの', `<ul>
      <li>左端の線はいつも main（シアン）。ほかの線の色は名前で決まります（テーマの ANSI の 16 色から、赤と灰を除いたもの）。</li>
      <li><b>⧉</b> は worktree の先頭。横の印はそこにいる Claude の様子（<span style="color:var(--accent)">●</span> 実行中・◐ 待ち・✓ 済み）。</li>
      <li>点線の白丸は、まだコミットしていない変更（WIP）。worktree ごとに 1 行です。</li>
      <li><b>⚠ 衝突の予測</b>は、worktree どうしを <code>git merge-tree</code> で合わせてみて、ぶつかると分かった先頭。点線の枠は、まだコミットしていない変更を数えてぶつかるもの（WIP の行に付きます）。</li>
      <li><b>PR を畳む</b>（既定で入、<kbd>z</kbd>）: main に入ったマージは、main の first-parent の 1 行だけにします。行の <span class="fchip" style="--c:var(--t-wait)">▸ 7</span> は畳んだコミットの数で、押すと開きます。左の短い線は、畳んだ枝が入ってきた印です。worktree と、まだ main に入っていないブランチはいつも出たままです。</li>
      <li>青い点と太字は、まだ見ていないコミット。選ぶと消え、worktree の表の数も減ります。</li></ul>`],
    ['動かしかた', `<ul>
      <li>窓を押してから <kbd>j</kbd> <kbd>k</kbd> で動かし、<kbd>Enter</kbd>（またはダブルクリック）で差分を横並びに広げます。<kbd>Esc</kbd> で戻ります。</li>
      <li><kbd>/</kbd> で絞ります。合わないコミットは薄くなり、右の細いバーに印が付きます（<kbd>Enter</kbd> で次）。</li>
      <li>右の細いバーが樹形図のただ 1 つのスクロールの印です。worktree の先頭・WIP・予測・タグ・選んでいる所が並び、押すとそこへ飛びます。</li>
      <li>下の「… さらに古い N 件」は押すと読み込みます。左の feat/・fix/ 以外の束も同じく畳んであります。</li>
      <li>マージの行で <kbd>l</kbd> が開く・<kbd>h</kbd> が畳む。右の欄に「このマージで main に入った N コミット」が並び、押すとそのコミットへ飛びます（畳んだ中なら開きます）。左のブランチを押したときも同じです。</li>
      <li><kbd>N</kbd> で新しい作業、<kbd>?</kbd> でキーの一覧。</li></ul>`],
    ['決まったこと（2026-10-15）', `<ul>
      <li>lane の PR を畳む切り替えを付ける。既定で入で、main の first-parent だけを見せます。</li>
      <li>Claude が書いている途中（● 実行中）の WIP も、「変更」（2）でステージできます。止めずに、ステージとコミットの前に「Claude が書いている途中です」と出して確かめます。</li>
      <li>差分の中の行のステージは「変更」（2）だけ。樹形図では読むだけです。</li></ul>`],
  ],
  wt: [
    ['この表で見るもの', `<ul>
      <li>1 行が 1 つの worktree。1 つの窓が 1 つのリポジトリ（全部の worktree を含む）を持ちます。</li>
      <li>Claude の列は tsumugi の <code>tsumugi ls --json</code> から。待ち（◐）の行には、Claude が聞いていることがそのまま出ます。</li>
      <li>「タブへ」（<kbd>o</kbd>）は <code>tsumugi focus N</code>。tsumugi の窓が前に出て、そのタブが選ばれます（TODO に足したもの）。</li>
      <li>幅 960 では表がカードになります。</li></ul>`],
    ['ぶつかりの予測', `<ul>
      <li>2 本ずつ <code>git merge-tree --write-tree</code> で合わせてみます。作業フォルダには触りません。</li>
      <li>new-worktree × focus が 4 か所。どちらかを main に入れると、もう片方を入れるときに「衝突の解消」になります（場面「衝突の解消」）。</li>
      <li>未コミットの変更も数えます（2026-10-15 に決定）。作業フォルダの中身から一時の tree を作り、ブランチも index も動かしません。
        fix/142 の未コミットの版の繰り上げと CHANGELOG.md が、ほかの 2 本と 2 か所ずつぶつかります（黄の点線の枠）。</li></ul>`],
    ['新しい作業（N）', `<ul>
      <li>ブランチ・元・指示・本数を入れると、<code>tsumugi new --worktree … --from …</code> を回します（TODO に足したもの）。</li>
      <li>本数を 2 以上にすると、同じ指示で並べて始めます（tsumugi の「並べて始める」と同じ）。</li></ul>`],
  ],
  cf: [
    ['この画面で見るもの', `<ul>
      <li>feat/new-worktree を main に入れたあと、feat/focus を入れて 4 か所で止まった所です。</li>
      <li>3 列は 自分（main）・base（分かれた所）・相手（feat/focus）。上の行は、そこを変えたコミット（<code>git log -L</code> で引く）。意図が分かると選びやすくなります。</li>
      <li>2 つ目の箇所は、前に同じ形を解いたので rerere が当ててあります。「確かめた」を押すまで印が残ります。</li>
      <li>結果の欄はそのまま書き直せます。ボタンは結果を埋めるだけです。</li></ul>`],
    ['動かしかた', `<ul>
      <li><kbd>]</kbd> <kbd>[</kbd> で箇所を動き、<kbd>o</kbd> 自分・<kbd>t</kbd> 相手・<kbd>b</kbd> <kbd>B</kbd> 両方。選ぶと次のまだの箇所へ進みます。</li>
      <li><kbd>e</kbd> で結果を直し、<kbd>Esc</kbd> で抜けます。全部解けると下にコミットの欄が出ます。</li>
      <li>「Claude に渡す」は <code>tsumugi send</code> で、残りの箇所を作者の Claude に頼みます。「マージ全体を中止」は確かめてから。</li>
      <li>幅 960 では base の列を隠し、各箇所の「base を見る」で出します。</li></ul>`],
    ['決まったこと（2026-10-15）', `<ul>
      <li>Claude に渡したあとは、ファイルの見張り（300 ms）で衝突の印を読み直し、残りの数を減らしていきます。<code>tsumugi ls --json</code> で
        その Claude が待ち・済みになったら知らせ、確かめるのは人です。<code>tsumugi wait</code> では止まりません。</li>
      <li>マージ中は、その worktree で HEAD と index を動かす操作（pull・checkout・rebase・merge・cherry-pick・stash・reset、マージ以外のコミット）を止めます。
        読むこと・fetch・push・ほかの worktree の操作（新しい作業など）はそのまま。タイトルバーに「マージ中」が出て、止めたボタンは薄くなります。</li>
      <li>rerere は、最初に衝突したときに聞いて、このリポジトリの設定（<code>rerere.enabled</code>）に書きます。</li></ul>`],
  ],
  logo: [
    ['決まったこと', `<ul>
      <li>ロゴは A 合流（2026-10-15）。B 道標と C 道は使いません。</li>
      <li>字は michi だけ（道の字は入れない）。字の形は窓と同じ Segoe UI の太字です。</li>
      <li>アプリのアイコンは、kura と同じ tsumugi Dark の暗い板に印を載せたもの。正はリポの <code>assets/icon.svg</code> で、この画面もそれを読んでいます。</li></ul>`],
    ['色', '<p>窓の中ではテーマの run（シアン、main の線と同じ）と wait（黄）を使い、テーマを変えると窓のタイトルバーの印が追いかけます。アイコンは tsumugi Dark の色で固定です（地 <code>#16181d</code>、main の線 <code>#6fd0d0</code>、合流する線 <code>#e8c87a</code>）。</p>'],
    ['この先', '<p>Windows の <code>.ico</code> は、コードを始めるとき（v0.1.0）に kura と同じ手順で SVG から作ります。窓のアイコンは起動のときに SVG から描きます。</p>'],
  ],
};
function renderNotes() {
  $('#notes').innerHTML = NOTES[S.scene].map(([h, b]) => `<div class="note"><h3>${h}</h3>${b}</div>`).join('');
}

/* While a merge is open, what moves HEAD or the index of that worktree waits. */
const LOCKED = 'マージ中なので、この worktree（main）では止めています。解き終わるか、マージを中止すると使えます。';
function syncLocks() {
  for (const b of document.querySelectorAll('#v-graph .tool [data-act="pull"], #v-graph .tool [data-act="stash"]')) {
    b.classList.toggle('locked', S.merging);
    b.title = S.merging ? 'マージ中は止めています' : '';
  }
}

function setScene(s) {
  S.scene = s;
  S.merging = s === 'cf';
  S.wide = false;
  S.filter = '';
  S.limit = 64;
  S.mx = false;
  S.unfold.clear();
  syncFold();
  syncLocks();
  $('#filter').value = '';
  $('#v-graph').classList.remove('wide');
  cfInit();
  G = computeGraph();
  S.sel = S.merging ? 0 : Math.max(0, G.rows.findIndex(r => r.id === '71c4a2f'));
  $('#logos').hidden = s !== 'logo';
  renderSide();
  drawGraph();
  renderDetail();
  renderWt();
  $('#v-cf').innerHTML = '';
  renderCf();
  renderNotes();
  syncSeg();
  setView(s === 'wt' ? 'wt' : s === 'cf' ? 'cf' : 'graph');
  $('#glist').scrollTop = 0;
}

/* ── wiring ───────────────────────────────────────────────── */
function init() {
  const win = $('#win');
  applyTheme();
  layout();
  renderLogos();
  const hash = location.hash.replace('#', '');
  setScene(SCENES.includes(hash) ? hash : 'graph');

  $('#controls').addEventListener('click', e => {
    const b = e.target.closest('.seg button');
    if (!b) return;
    const ctl = b.closest('.seg').dataset.ctl, v = b.dataset.v;
    S[ctl] = v;
    syncSeg();
    if (ctl === 'scene') setScene(v);
    if (ctl === 'theme') { applyTheme(); store.set('theme', v); }
    if (ctl === 'width' || ctl === 'fit') {
      layout();
      if (S.merging) renderCf();
      requestAnimationFrame(() => { drawBar(); ensureVisible(S.sel); });
    }
  });
  win.addEventListener('click', onClick);
  win.addEventListener('dblclick', e => { const r = e.target.closest('.grow[data-i]'); if (r) setWide(true); });
  win.addEventListener('mousedown', e => {
    if (e.target.closest('input, textarea, select, [contenteditable="true"], .gbar')) return;
    if (!win.contains(document.activeElement) || document.activeElement === document.body) win.focus({ preventScroll: true });
  });
  document.addEventListener('keydown', onKey);
  $('#filter').addEventListener('input', onFilter);
  $('#glist').addEventListener('scroll', moveThumb, { passive: true });
  $('#ov').addEventListener('input', e => { if (e.target.closest('#nw-br, #nw-from, #nw-task')) nwPreview(); });
  $('#ov').addEventListener('change', e => { if (e.target.id === 'nw-from') nwPreview(); });
  $('#v-cf').addEventListener('change', e => {
    if (e.target.id === 'cf-to') { S.cf.to = e.target.value; $('#cf-cmd').textContent = cfCmd(); }
  });
  $('#v-cf').addEventListener('input', e => {
    const ed = e.target.closest('.res-b');
    if (!ed) return;
    const bi = +ed.dataset.bi, st = S.cf.blocks[bi];
    const text = ed.innerText.replace(/\n$/, '');
    Object.assign(st, { res: text.split('\n'), tag: 'hand', done: text.trim() !== '', checked: true });
    if (!st.done) Object.assign(st, { res: null, tag: null });
    cfRefresh(bi);
  });
  $('#gbar').addEventListener('mousedown', e => {
    const list = $('#glist'), bar = $('#gbar');
    const total = list.scrollHeight, bh = bar.clientHeight;
    const mk = e.target.closest('.mk');
    e.preventDefault();
    if (mk) { selectRow(+mk.dataset.i); return; }
    const th = e.target.closest('.gthumb');
    if (!th) {
      const y = (e.clientY - bar.getBoundingClientRect().top) / S.k;
      list.scrollTop = y / bh * total - list.clientHeight / 2;
      return;
    }
    th.classList.add('drag');
    const y0 = e.clientY, s0 = list.scrollTop;
    const move = ev => { list.scrollTop = s0 + (ev.clientY - y0) / S.k * total / bh; };
    const up = () => { th.classList.remove('drag'); removeEventListener('mousemove', move); removeEventListener('mouseup', up); };
    addEventListener('mousemove', move);
    addEventListener('mouseup', up);
  });
  let rt = 0;
  addEventListener('resize', () => {
    cancelAnimationFrame(rt);
    rt = requestAnimationFrame(() => { layout(); drawBar(); });
  });
}

init();
})();
