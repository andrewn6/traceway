import { IncomingMessage } from "node:http";

export function pathSegments(req: IncomingMessage): string[] {
  return new URL(req.url ?? "/", "http://local").pathname.split("/").filter(Boolean);
}
