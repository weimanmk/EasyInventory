export function money(value?: number) {
  return `¥${Number(value ?? 0).toFixed(2)}`;
}

export function qty(value?: number) {
  return Number(value ?? 0).toLocaleString('zh-CN', { maximumFractionDigits: 2 });
}

export function uniqueValues<T>(rows: T[], selector: (row: T) => string | undefined) {
  return Array.from(new Set(rows.map(selector).filter(Boolean) as string[]));
}
