const { writeFileSync } = require("fs");
const { deflateSync } = require("zlib");

function createPNG(width, height) {
  const raw = [];
  for (let y = 0; y < height; y++) {
    raw.push(0);
    for (let x = 0; x < width; x++) {
      raw.push(102, 126, 234, 255);
    }
  }
  const compressed = deflateSync(Buffer.from(raw));
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  function crc32(buf) {
    let c = 0xffffffff;
    for (let i = 0; i < buf.length; i++) {
      c ^= buf[i];
      for (let j = 0; j < 8; j++) c = (c >>> 1) ^ (c & 1 ? 0xedb88320 : 0);
    }
    return c ^ 0xffffffff;
  }

  function chunk(type, data) {
    const t = Buffer.from(type);
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([t, data])) >>> 0);
    return Buffer.concat([len, t, data, crc]);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; ihdr[9] = 6;

  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", compressed), chunk("IEND", Buffer.alloc(0))]);
}

const base = "src-tauri/icons";
const { mkdirSync, existsSync } = require("fs");
if (!existsSync(base)) mkdirSync(base, { recursive: true });

[[32,"32x32.png"],[128,"128x128.png"],[256,"128x128@2x.png"]].forEach(([s,f]) => {
  writeFileSync(base+"/"+f, createPNG(s,s));
  console.log("Created", f);
});
writeFileSync(base+"/icon.ico", createPNG(256,256));
writeFileSync(base+"/icon.icns", createPNG(512,512));
console.log("Done");
