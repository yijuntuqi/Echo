// Generates the pet's 30 SVG assets (5 stages x 6 states) into public/.
// Deterministic: same code, same bytes. Designers can hand-edit any file
// afterwards — re-running this script simply overwrites them all.
//
//   node scripts/gen-pet-assets.mjs
//
// Naming follows the convention petStore.svgPaths already derives:
//   public/<stage>/<state>.svg   e.g. public/egg/idle.svg

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', 'public');

const STAGES = ['egg', 'child', 'teen', 'adult', 'ultimate'];
const STATES = ['idle', 'walk', 'sleep', 'talk', 'react', 'evolve'];

/** Palette per stage, continuous with the old inline-art colors. */
const PALETTE = {
  egg:      { body: '#FFF8DC', accent: '#E8D9A0', deep: '#C9B374', cheeks: 'rgba(214,142,90,0.35)' },
  child:    { body: '#FFE4B5', accent: '#E6B980', deep: '#C2955B', cheeks: 'rgba(214,120,90,0.35)' },
  teen:     { body: '#FFD700', accent: '#D4A900', deep: '#A8880A', cheeks: 'rgba(214,100,80,0.35)' },
  adult:    { body: '#FF8C00', accent: '#D16F00', deep: '#A85400', cheeks: 'rgba(200,80,60,0.35)' },
  ultimate: { body: '#FF4500', accent: '#C23600', deep: '#8F2500', cheeks: 'rgba(160,50,40,0.40)' },
};

/** Body geometry per stage: grown across evolution, all inside the
 * window's hit-test ellipse (roughly x 30..170, y 10..184). */
const GEOMETRY = {
  egg:      { cx: 100, cy: 90, rx: 56, ry: 64, top: 0.74 },
  child:    { cx: 100, cy: 96, rx: 52, ry: 56, top: 0.80 },
  teen:     { cx: 100, cy: 92, rx: 54, ry: 60, top: 0.76 },
  adult:    { cx: 100, cy: 90, rx: 58, ry: 62, top: 0.74 },
  ultimate: { cx: 100, cy: 88, rx: 60, ry: 63, top: 0.72 },
};

const INK = '#333333';

/** Egg-like body: top narrower than bottom, four cubic segments. */
function bodyPath({ cx, cy, rx, ry, top }) {
  const t = ry * top;
  return [
    `M ${cx} ${cy - ry}`,
    `C ${cx + rx * 0.61} ${cy - ry} ${cx + rx} ${cy - t} ${cx + rx} ${cy + ry * 0.12}`,
    `C ${cx + rx} ${cy + ry * 0.78} ${cx + rx * 0.57} ${cy + ry} ${cx} ${cy + ry}`,
    `C ${cx - rx * 0.57} ${cy + ry} ${cx - rx} ${cy + ry * 0.78} ${cx - rx} ${cy + ry * 0.12}`,
    `C ${cx - rx} ${cy - t} ${cx - rx * 0.61} ${cy - ry} ${cx} ${cy - ry} Z`,
  ].join(' ');
}

const shadow = (rx) =>
  `<ellipse cx="100" cy="170" rx="${rx}" ry="8" fill="rgba(0,0,0,0.10)"/>`;

const eyesOpen = (dx = 0) =>
  `<g fill="${INK}">
     <circle cx="${84 + dx}" cy="96" r="5"/><circle cx="${116 + dx}" cy="96" r="5"/>
   </g>
   <g fill="#ffffff">
     <circle cx="${86 + dx}" cy="94" r="1.6"/><circle cx="${118 + dx}" cy="94" r="1.6"/>
   </g>`;

const eyesClosed = `
  <path d="M78 96 q6 6 12 0 M110 96 q6 6 12 0" stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round"/>`;

const eyesHappy = `
  <path d="M78 100 q6 -8 12 0 M110 100 q6 -8 12 0" stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round"/>`;

const eyeStar = (cx, cy) => {
  const pts = [];
  for (let i = 0; i < 8; i++) {
    const r = i % 2 === 0 ? 7 : 3;
    const a = (Math.PI / 4) * i - Math.PI / 2;
    pts.push(`${(cx + r * Math.cos(a)).toFixed(1)},${(cy + r * Math.sin(a)).toFixed(1)}`);
  }
  return `<polygon points="${pts.join(' ')}" fill="${INK}"/>`;
};

const smile = `<path d="M88 116 q12 10 24 0" stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round"/>`;
const mouthO = `<ellipse cx="100" cy="119" rx="6" ry="8" fill="${INK}"/><ellipse cx="100" cy="121" rx="3" ry="4" fill="#e2707a"/>`;
const mouthTiny = `<path d="M94 119 h12" stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round"/>`;
const blush = (p) =>
  `<ellipse cx="70" cy="112" rx="7" ry="4" fill="${p.cheeks}"/><ellipse cx="130" cy="112" rx="7" ry="4" fill="${p.cheeks}"/>`;

/** Stage-specific body decorations, drawn under the face. */
function stageDeco(stage, p) {
  switch (stage) {
    case 'egg':
      return `<g fill="${p.accent}" opacity="0.6">
        <ellipse cx="82" cy="60" rx="6" ry="4"/>
        <ellipse cx="118" cy="74" rx="5" ry="3.4"/>
        <ellipse cx="90" cy="132" rx="6" ry="4"/>
      </g>`;
    case 'child':
      return `<g fill="${p.body}" stroke="${p.accent}" stroke-width="3">
        <ellipse cx="44" cy="112" rx="9" ry="14" transform="rotate(18 44 112)"/>
        <ellipse cx="156" cy="112" rx="9" ry="14" transform="rotate(-18 156 112)"/>
      </g>`;
    case 'teen':
      return `<g fill="${p.body}" stroke="${p.accent}" stroke-width="3" stroke-linejoin="round">
        <path d="M62 40 l8 -16 8 14"/>
        <path d="M122 38 l8 -14 8 16"/>
      </g>${blush(p)}`;
    case 'adult':
      return `<g stroke="${p.accent}" stroke-width="4" fill="none" stroke-linecap="round">
        <path d="M92 28 q-2 -12 8 -16"/>
        <path d="M108 28 q2 -12 -8 -16"/>
      </g>${blush(p)}`;
    case 'ultimate':
      return `<g>
        <path d="M100 24 q-10 -16 -2 -22 q10 6 6 14 q8 -8 14 -2 q-2 8 -10 12 Z" fill="${p.accent}"/>
        <circle cx="100" cy="90" r="72" fill="none" stroke="${p.accent}" stroke-width="2" stroke-dasharray="6 10" opacity="0.5"/>
        <g fill="${p.deep}" opacity="0.8">
          <path d="M34 62 l4 -8 4 8 -4 8 Z"/>
          <path d="M162 118 l4 -8 4 8 -4 8 Z"/>
          <path d="M148 40 l3 -6 3 6 -3 6 Z"/>
        </g>
      </g>`;
  }
}

/** State-specific composition: eyes, mouth, pose, effects. */
function stateArt(state) {
  switch (state) {
    case 'idle':
      return { face: eyesOpen() + smile, wrap: (inner) => inner, extra: '' };
    case 'walk':
      return {
        face: eyesOpen(3) + smile,
        wrap: (inner) => `<g transform="rotate(-7 100 140)">${inner}</g>`,
        extra: `<g stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round" opacity="0.5">
                  <path d="M30 120 q-8 6 -2 14"/><path d="M20 108 q-10 8 -3 18"/>
                </g>`,
      };
    case 'sleep':
      return {
        face: eyesClosed + mouthTiny,
        wrap: (inner) => inner,
        extra: `<g stroke="#8aa2c0" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M146 52 h12 l-12 12 h12"/>
                  <path d="M164 34 h9 l-9 9 h9" opacity="0.7"/>
                </g>`,
      };
    case 'talk':
      return {
        face: eyesOpen() + mouthO,
        wrap: (inner) => inner,
        extra: `<g fill="${INK}" opacity="0.75">
                  <circle cx="150" cy="46" r="4"/><circle cx="162" cy="36" r="5.5"/><circle cx="176" cy="24" r="7"/>
                </g>`,
      };
    case 'react':
      return {
        face: eyeStar(84, 96) + eyeStar(116, 96) + `<path d="M86 116 q14 12 28 0" stroke="${INK}" stroke-width="3" fill="none" stroke-linecap="round"/>`,
        wrap: (inner) => `<g transform="translate(0 -3)">${inner}</g>`,
        extra: `<g fill="${INK}">
                  <rect x="140" y="34" width="6" height="18" rx="3"/>
                  <circle cx="143" cy="60" r="3.4"/>
                </g>`,
      };
    case 'evolve':
      return {
        face: eyesHappy + smile,
        wrap: (inner) => inner,
        extra: `<g stroke="#f5b942" stroke-width="3.5" stroke-linecap="round" opacity="0.9">
                  ${Array.from({ length: 8 }, (_, i) => {
                    const a = (Math.PI / 4) * i - Math.PI * 0.94;
                    const x1 = 100 + 78 * Math.cos(a), y1 = 88 + 78 * Math.sin(a);
                    const x2 = 100 + 92 * Math.cos(a), y2 = 88 + 92 * Math.sin(a);
                    return `<line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/>`;
                  }).join('')}
                </g>
                <g fill="#ffd700">
                  <path d="M52 34 l3.5 -7 3.5 7 -3.5 7 Z"/>
                  <path d="M158 60 l3 -6 3 6 -3 6 Z"/>
                </g>`,
      };
  }
}

let written = 0;
for (const stage of STAGES) {
  mkdirSync(join(ROOT, stage), { recursive: true });
  for (const state of STATES) {
    const p = PALETTE[stage];
    const g = GEOMETRY[stage];
    const s = stateArt(state);
    const inner = [
      shadow(g.rx * 0.9),
      stageDeco(stage, p),
      `<path d="${bodyPath(g)}" fill="${p.body}" stroke="${p.accent}" stroke-width="3"/>`,
      s.face,
    ].join('\n      ');
    const svg = `<!-- Generated by scripts/gen-pet-assets.mjs — hand edits are fine; rerunning overwrites. -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
  ${s.wrap(inner)}
  ${s.extra}
</svg>
`;
    writeFileSync(join(ROOT, stage, `${state}.svg`), svg);
    written++;
  }
}
console.log(`wrote ${written} SVGs under ${ROOT}`);
