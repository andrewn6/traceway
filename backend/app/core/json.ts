export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export function asJson(value: unknown): JsonValue {
  return value as JsonValue;
}

export function asOptionalJson(value: unknown): JsonValue | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return value as JsonValue;
}
