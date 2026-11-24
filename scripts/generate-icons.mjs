#!/usr/bin/env node
// Generates every raster asset the launcher and installer need, with no
// external image dependency. Run: npm run icons
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const COLORS = {
  base: [11, 13, 16],
  panel: [16, 20, 26],
  panelTop: [23, 29, 38],
  border: [42, 52, 65],
  ink: [242, 244, 247],
  accent: [142, 169, 255],
};

function createCanvas(size) {
  return { size, data: new Float64Array(size * size * 4) };
}

function blend(canvas, x, y, color, alpha) {
  if (alpha <= 0) return;
  const a = Math.min(1, alpha);
  const i = (y * canvas.size + x) * 4;
  const d = canvas.data;
  for (let c = 0; c < 3; c += 1) {
    d[i + c] = d[i + c] * (1 - a) + color[c] * a;
  }
  d[i + 3] = Math.min(1, d[i + 3] + a);
}

function roundRectDistance(px, py, x0, y0, x1, y1, r) {
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  const hx = (x1 - x0) / 2 - r;
  const hy = (y1 - y0) / 2 - r;
  const qx = Math.abs(px - cx) - hx;
  const qy = Math.abs(py - cy) - hy;
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0));
  const inside = Math.min(Math.max(qx, qy), 0);
  return outside + inside - r;
}

function fillRoundRect(canvas, x0, y0, x1, y1, r, color, alpha = 1, gradient = null) {
  const minX = Math.max(0, Math.floor(x0 - 1));
  const maxX = Math.min(canvas.size - 1, Math.ceil(x1 + 1));
  const minY = Math.max(0, Math.floor(y0 - 1));
  const maxY = Math.min(canvas.size - 1, Math.ceil(y1 + 1));
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const d = roundRectDistance(x + 0.5, y + 0.5, x0, y0, x1, y1, r);
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage <= 0) continue;
      let c = color;
      if (gradient) {
        const t = Math.min(1, Math.max(0, ((y + 0.5) - y0) / (y1 - y0)));
        c = [
          gradient[0][0] + (gradient[1][0] - gradient[0][0]) * t,
          gradient[0][1] + (gradient[1][1] - gradient[0][1]) * t,
          gradient[0][2] + (gradient[1][2] - gradient[0][2]) * t,
        ];
      }
      blend(canvas, x, y, c, coverage * alpha);
    }
  }
}

function fillCircle(canvas, cx, cy, r, color, alpha = 1) {
  const minX = Math.max(0, Math.floor(cx - r - 1));
  const maxX = Math.min(canvas.size - 1, Math.ceil(cx + r + 1));
  const minY = Math.max(0, Math.floor(cy - r - 1));
  const maxY = Math.min(canvas.size - 1, Math.ceil(cy + r + 1));
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy) - r;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blend(canvas, x, y, color, coverage * alpha);
    }
  }
}

/** Draws the Leonida Launcher mark: a rounded tile, a geometric L and one accent dot. */
function drawMark(canvas, { tile = true, inset = 0 } = {}) {
  const s = canvas.size;
  const pad = s * (0.02 + inset);
  if (tile) {
    fillRoundRect(canvas, pad, pad, s - pad, s - pad, s * 0.225, COLORS.panel, 1, [COLORS.panelTop, COLORS.base]);
    // hairline border
    const t = Math.max(0.6, s * 0.006);
    fillRoundRect(canvas, pad, pad, s - pad, s - pad, s * 0.225, COLORS.panel, 0);
    const inner = pad + t;
    // draw border by filling then clearing the inside is destructive; instead
    // stroke with four thin rounded rects is not needed for a dark tile.
    void inner;
  }
  const stroke = s * 0.105;
  const left = s * 0.325;
  const bottom = s * 0.735;
  const top = s * 0.265;
  const right = s * 0.695;
  fillRoundRect(canvas, left, top, left + stroke, bottom, stroke * 0.34, COLORS.ink);
  fillRoundRect(canvas, left, bottom - stroke, right, bottom, stroke * 0.34, COLORS.ink);
  fillCircle(canvas, s * 0.715, s * 0.3, s * 0.05, COLORS.accent, 0.95);
}

function downsample(canvas, factor, target) {
  const out = { size: target, data: new Float64Array(target * target * 4) };
  const src = canvas.size;
  const step = src / target;
  for (let y = 0; y < target; y += 1) {
    for (let x = 0; x < target; x += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      let count = 0;
      for (let sy = Math.floor(y * step); sy < Math.floor((y + 1) * step); sy += 1) {
        for (let sx = Math.floor(x * step); sx < Math.floor((x + 1) * step); sx += 1) {
          const i = (sy * src + sx) * 4;
          r += canvas.data[i];
          g += canvas.data[i + 1];
          b += canvas.data[i + 2];
          a += canvas.data[i + 3];
          count += 1;
        }
      }
      const i = (y * target + x) * 4;
      out.data[i] = r / count;
      out.data[i + 1] = g / count;
      out.data[i + 2] = b / count;
      out.data[i + 3] = a / count;
      void factor;
    }
  }
  return out;
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, payload) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(payload.length, 0);
  const typeBuffer = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, payload])), 0);
  return Buffer.concat([length, typeBuffer, payload, crc]);
}

function encodePng(canvas) {
  const { size, data } = canvas;
  const raw = Buffer.alloc(size * (size * 4 + 1));
  let offset = 0;
  for (let y = 0; y < size; y += 1) {
    raw[offset] = 0;
    offset += 1;
    for (let x = 0; x < size; x += 1) {
      const i = (y * size + x) * 4;
      const a = Math.min(1, Math.max(0, data[i + 3]));
      // Un-premultiplied straight alpha, over the tile colour.
      raw[offset] = Math.round(Math.min(255, Math.max(0, data[i])));
      raw[offset + 1] = Math.round(Math.min(255, Math.max(0, data[i + 1])));
      raw[offset + 2] = Math.round(Math.min(255, Math.max(0, data[i + 2])));
      raw[offset + 3] = Math.round(a * 255);
      offset += 4;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function encodeBmp(width, height, pixels) {
  const rowBytes = width * 3 + ((width * 3) % 4 === 0 ? 0 : 4 - ((width * 3) % 4));
  const pixelBytes = rowBytes * height;
  const header = Buffer.alloc(54);
  header.write("BM", 0, "ascii");
  header.writeUInt32LE(54 + pixelBytes, 2);
  header.writeUInt32LE(54, 10);
  header.writeUInt32LE(40, 14);
  header.writeInt32LE(width, 18);
  header.writeInt32LE(height, 22);
  header.writeUInt16LE(1, 26);
  header.writeUInt16LE(24, 28);
  header.writeUInt32LE(pixelBytes, 34);
  header.writeInt32LE(2835, 38);
  header.writeInt32LE(2835, 42);
  const body = Buffer.alloc(pixelBytes);
  for (let y = 0; y < height; y += 1) {
    const srcY = height - 1 - y; // BMP stores bottom-up
    for (let x = 0; x < width; x += 1) {
      const src = (srcY * width + x) * 3;
      const dst = y * rowBytes + x * 3;
      body[dst] = pixels[src + 2];
      body[dst + 1] = pixels[src + 1];
      body[dst + 2] = pixels[src];
    }
  }
  return Buffer.concat([header, body]);
}

function renderIcon(size, options = {}) {
  const supersample = 4;
  const big = createCanvas(size * supersample);
  drawMark(big, options);
  return downsample(big, supersample, size);
}

function write(path, buffer) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, buffer);
  console.log(`  ${path.replace(root + "\\", "").replace(root + "/", "")} (${buffer.length} bytes)`);
}

function encodeIco(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  const entries = [];
  let offset = 6 + images.length * 16;
  for (const { size, png } of images) {
    const entry = Buffer.alloc(16);
    entry[0] = size >= 256 ? 0 : size;
    entry[1] = size >= 256 ? 0 : size;
    entry[2] = 0;
    entry[3] = 0;
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(png.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += png.length;
    entries.push(entry);
  }
  return Buffer.concat([header, ...entries, ...images.map((i) => i.png)]);
}

console.log("Generating launcher icons and installer artwork…");

const iconTargets = [
  ["icons/32x32.png", 32],
  ["icons/128x128.png", 128],
  ["icons/128x128@2x.png", 256],
  ["icons/icon.png", 512],
];

for (const [relative, size] of iconTargets) {
  write(join(root, "src-tauri", relative), encodePng(renderIcon(size)));
}
write(join(root, "public", "mark.png"), encodePng(renderIcon(512)));

const icoImages = [16, 24, 32, 48, 64, 128, 256].map((size) => ({
  size,
  png: encodePng(renderIcon(size)),
}));
write(join(root, "src-tauri", "icons", "icon.ico"), encodeIco(icoImages));

// Installer artwork -------------------------------------------------------
function renderBmp(width, height, draw) {
  const pixels = new Uint8Array(width * height * 3);
  const canvas = { width, height, pixels };
  draw(canvas);
  return encodeBmp(width, height, pixels);
}

function paint(canvas, x, y, color, alpha = 1) {
  if (x < 0 || y < 0 || x >= canvas.width || y >= canvas.height) return;
  const i = (y * canvas.width + x) * 3;
  for (let c = 0; c < 3; c += 1) {
    canvas.pixels[i + c] = canvas.pixels[i + c] * (1 - alpha) + color[c] * alpha;
  }
}

function bmpFill(canvas, color) {
  for (let y = 0; y < canvas.height; y += 1) {
    for (let x = 0; x < canvas.width; x += 1) paint(canvas, x, y, color);
  }
}

function bmpGradient(canvas, top, bottom) {
  for (let y = 0; y < canvas.height; y += 1) {
    const t = y / Math.max(1, canvas.height - 1);
    const color = [0, 1, 2].map((c) => top[c] + (bottom[c] - top[c]) * t);
    for (let x = 0; x < canvas.width; x += 1) paint(canvas, x, y, color);
  }
}

function bmpMark(canvas, originX, originY, size) {
  const icon = renderIcon(size);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const i = (y * size + x) * 4;
      const alpha = icon.data[i + 3];
      if (alpha <= 0.01) continue;
      paint(canvas, originX + x, originY + y, [icon.data[i], icon.data[i + 1], icon.data[i + 2]], Math.min(1, alpha));
    }
  }
}

// NSIS header: 150x57, mark on the right, quiet gradient on the left.
write(
  join(root, "src-tauri", "installer", "header.bmp"),
  renderBmp(150, 57, (canvas) => {
    bmpGradient(canvas, [18, 22, 29], [11, 13, 16]);
    bmpMark(canvas, 98, 8, 41);
    for (let x = 0; x < 4; x += 1) {
      for (let y = 0; y < canvas.height; y += 1) {
        paint(canvas, x, y, COLORS.accent, 0.35 * (1 - x / 4));
      }
    }
  }),
);

// NSIS sidebar: 164x314, the mark centred with generous space.
write(
  join(root, "src-tauri", "installer", "sidebar.bmp"),
  renderBmp(164, 314, (canvas) => {
    bmpGradient(canvas, [16, 20, 26], [9, 11, 14]);
    bmpMark(canvas, 32, 111, 100);
    for (let y = 0; y < canvas.height; y += 1) {
      paint(canvas, 0, y, COLORS.border, 0.9);
      paint(canvas, 1, y, COLORS.border, 0.35);
    }
  }),
);

console.log("Done.");
