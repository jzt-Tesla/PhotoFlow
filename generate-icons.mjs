// Generate simple placeholder PNG icons for Tauri
import { writeFileSync } from "fs";

function createPNG(width, height) {
  // Minimal valid PNG: a solid purple square
  const { deflateSync } = await import("zlib");

  // Build raw RGBA pixel data
  const raw = [];
  for (let y = 0; y < height; y++) {
    raw.push(0); // filter byte
    for (let x = 0; x < width; x++) {
      // Purple gradient
      raw.push(102, 126, 234, 255); // RGBA
    }
  }

  const compressed = deflateSync(Buffer.from(raw));

  // PNG signature
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  function chunk(type, data) {
    const typeB = Buffer.from(type);
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const crcData = Buffer.concat([typeB, data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(crcData) >>> 0);
    return Buffer.concat([len, typeB, data, crc]);
  }

  // CRC32 implementation
  function crc32(buf) {
    let crc = 0xffffffff;
    for (let i = 0; i < buf.length; i++) {
      crc ^= buf[i];
      for (let j = 0; j < 8; j++) {
        crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
      }
    }
    return crc ^ 0xffffffff;
  }

  // IHDR
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0; // compression
  ihdr[11] = 0; // filter
  ihdr[12] = 0; // interlace

  // IEND
  const iend = Buffer.alloc(0);

  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", compressed),
    chunk("IEND", iend),
  ]);
}

// Generate icons
const sizes = [
  [32, "icons/32x32.png"],
  [128, "icons/128x128.png"],
  [256, "icons/128x128@2x.png"],
];

for (const [size, path] of sizes) {
  writeFileSync(path, createPNG(size, size));
  console.log(`Created ${path}`);
}

// Copy 128x128 as icon.ico (simplified - real ICO would need proper format)
// For Tauri dev mode, a renamed PNG works on Windows
writeFileSync("icons/icon.ico", createPNG(256, 256));
console.log("Created icons/icon.ico");

writeFileSync("icons/icon.icns", createPNG(512, 512));
console.log("Created icons/icon.icns");
