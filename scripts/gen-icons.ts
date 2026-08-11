// Generates the app icon set for Tauri (PNG only: deb/appimage targets).
// No dependencies: renders the design into raw RGBA and encodes PNG manually.
//
// Usage: bun ./scripts/gen-icons.ts

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const OUT_DIR = join(import.meta.dir, "..", "src-tauri", "crates", "app", "icons");

const SIZES = [
  { name: "32x32.png", size: 32 },
  { name: "128x128.png", size: 128 },
  { name: "128x128@2x.png", size: 256 },
  { name: "icon.png", size: 512 },
];

// Theme colors (oklch -> sRGB), from apps/desktop/src/styles/app.css.
const BG = oklch_to_srgb(0.129, 0.042, 264.695); // slate-950-ish
const PANEL = oklch_to_srgb(0.208, 0.042, 265.755); // slate-800-ish
const ACCENT = oklch_to_srgb(0.746, 0.16, 232.661); // --color-accent
const ACCENT_DARK = oklch_to_srgb(0.5, 0.14, 232.661);
const TEXT = oklch_to_srgb(0.968, 0.003, 264.542); // slate-100-ish

function oklch_to_srgb(L: number, C: number, h: number): [number, number, number] {
  const hRad = (h * Math.PI) / 180;
  const a = C * Math.cos(hRad);
  const b = C * Math.sin(hRad);
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;
  return [
    clamp8(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    clamp8(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    clamp8(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  ];
}

function clamp8(v: number): number {
  return Math.round(Math.min(1, Math.max(0, v)) * 255);
}

function mix(a: number[], b: number[], t: number): [number, number, number] {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

function in_rounded_rect(x: number, y: number, rx: number, ry: number, w: number, h: number, r: number): boolean {
  if (x < rx || x >= rx + w || y < ry || y >= ry + h) return false;
  const dx = Math.max(rx + r - x, x - (rx + w - 1 - r), 0);
  const dy = Math.max(ry + r - y, y - (ry + h - 1 - r), 0);
  return dx * dx + dy * dy <= r * r;
}

function in_triangle(px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number): boolean {
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

function render(size: number): Buffer {
  const px = Buffer.alloc(size * size * 4);
  const set = (x: number, y: number, c: number[]) => {
    const i = (y * size + x) * 4;
    px[i] = c[0];
    px[i + 1] = c[1];
    px[i + 2] = c[2];
    px[i + 3] = 255;
  };

  const pad = size * 0.08;
  const boardX = pad;
  const boardY = pad;
  const boardW = size - pad * 2;
  const boardH = size - pad * 2;
  const boardR = size * 0.1;

  const gap = size * 0.03;
  const cellW = (boardW - gap * 2) / 3;
  const cellH = (boardH - gap * 2) / 3;
  const cellR = size * 0.04;

  // "Playing" button is the center cell.
  const playingCol = 1;
  const playingRow = 1;

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let color: number[] = BG;

      if (in_rounded_rect(x, y, boardX, boardY, boardW, boardH, boardR)) {
        for (let row = 0; row < 3; row++) {
          for (let col = 0; col < 3; col++) {
            const cx = boardX + gap + col * (cellW + gap);
            const cy = boardY + gap + row * (cellH + gap);
            if (!in_rounded_rect(x, y, cx, cy, cellW, cellH, cellR)) continue;

            const playing = row === playingRow && col === playingCol;
            if (playing) {
              const t = Math.min(1, (y - cy) / cellH);
              color = mix(PANEL, ACCENT, t);
            } else {
              const t = Math.min(1, (y - cy) / cellH);
              color = mix(BG, PANEL, t * 0.9);
            }

            // Play triangle in the center button.
            if (playing) {
              const tc = 0.32;
              const triW = cellW * tc;
              const triH = cellH * tc;
              const ax = cx + cellW * 0.42;
              const ay = cy + (cellH - triH) / 2;
              if (in_triangle(x, y, ax, ay, ax, ay + triH, ax + triW, ay + triH / 2)) {
                color = TEXT;
              }
            }
            break;
          }
        }
      }

      // Soft 1px border between the board and the background.
      set(x, y, color);
    }
  }

  return encode_png(size, px);
}

// --- Minimal PNG encoder (RGBA, 8-bit) ---

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

function encode_png(size: number, rgba: Buffer): Buffer {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  ihdr[10] = 0; // compression
  ihdr[11] = 0; // filter
  ihdr[12] = 0; // interlace

  const stride = size * 4;
  const raw = Buffer.alloc((stride + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

mkdirSync(OUT_DIR, { recursive: true });
for (const { name, size } of SIZES) {
  const out = join(OUT_DIR, name);
  writeFileSync(out, render(size));
  console.log(`wrote ${dirname(out)}/${name} (${size}x${size})`);
}
