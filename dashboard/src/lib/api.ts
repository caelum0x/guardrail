export const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`API request failed: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export async function getJsonOrNull<T>(path: string): Promise<T | null> {
  try {
    return await getJson<T>(path);
  } catch {
    return null;
  }
}

export async function getTextOrNull(path: string): Promise<string | null> {
  try {
    const response = await fetch(`${API_URL}${path}`, { cache: "no-store" });
    if (!response.ok) {
      return null;
    }
    return response.text();
  } catch {
    return null;
  }
}
