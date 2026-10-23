/** Browser-local receipt time, independent of locale display preferences. */
export function eventTimestamp(value: number) {
  const date = new Date(value);
  const pad = (part: number) => String(part).padStart(2, "0");
  const day = `${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  const clock = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  return {
    day,
    clock,
    full: `${String(date.getFullYear()).padStart(4, "0")}-${day} ${clock}:${pad(date.getSeconds())}`,
  };
}
