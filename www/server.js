import { createServer } from "http";
import { readFileSync, existsSync, statSync } from "fs";
import { join, extname } from "path";

const PORT = parseInt(process.env.PORT || "3000", 10);
const DIR = join(process.cwd(), "build");

const MIME = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".webp": "image/webp",
  ".webm": "video/webm",
  ".mp4": "video/mp4",
  ".txt": "text/plain",
  ".xml": "application/xml",
};

function serve(res, filePath) {
  try {
    if (!existsSync(filePath) || !statSync(filePath).isFile()) return false;
    const ext = extname(filePath);
    const mime = MIME[ext] || "application/octet-stream";
    const body = readFileSync(filePath);
    res.writeHead(200, {
      "Content-Type": mime,
      "Content-Length": body.length,
      "Cache-Control":
        ext === ".html" ? "no-cache" : "public, max-age=31536000, immutable",
    });
    res.end(body);
    return true;
  } catch {
    return false;
  }
}

const server = createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  let pathname = decodeURIComponent(url.pathname);

  // Try exact file
  if (serve(res, join(DIR, pathname))) return;

  // Try with .html
  if (serve(res, join(DIR, pathname + ".html"))) return;

  // Try index.html in directory
  if (serve(res, join(DIR, pathname, "index.html"))) return;

  // SPA fallback
  if (serve(res, join(DIR, "index.html"))) return;

  res.writeHead(404);
  res.end("Not Found");
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(`Listening on http://0.0.0.0:${PORT}`);
});
