export function logTime(timestampMs: number) {
  const value = new Date(timestampMs);
  const pad = (part: number) => String(part).padStart(2, '0');
  const date = `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}`;
  const clock = `${pad(value.getHours())}:${pad(value.getMinutes())}:${pad(value.getSeconds())}`;
  return { date, clock, text: `${date} ${clock}`, iso: value.toISOString() };
}
