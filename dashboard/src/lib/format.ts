export function usd(value: number): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(value);
}

export function usdString(value: string | number | undefined | null): string {
  if (value === undefined || value === null || value === "") {
    return "$0.00";
  }
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) {
    return "$0.00";
  }
  return usd(numeric);
}

export function pctString(value: string | number | undefined | null): string {
  if (value === undefined || value === null || value === "") {
    return "0.00%";
  }
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) {
    return "0.00%";
  }
  return `${numeric.toFixed(2)}%`;
}

export function compactDate(value: string | undefined): string {
  if (!value) {
    return "Pending";
  }
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) {
    return value;
  }
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

export function labelEvent(eventType: string): string {
  return eventType.replaceAll("_", " ");
}
