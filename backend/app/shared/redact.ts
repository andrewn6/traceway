export function apiKeyPreview(key: string | undefined): string | null {
  if (!key) return null;
  if (key.length <= 6) return key;
  return `${key.slice(0, 4)}...${key.slice(-2)}`;
}
