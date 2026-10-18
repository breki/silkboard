// One request at a time, paused while hidden and cancelled on teardown.
// eslint-disable-next-line no-unused-vars -- This parameter names a TypeScript callback argument.
export function poll(load: (signal: AbortSignal) => Promise<void>) {
  let stopped = false;
  let active: AbortController | null = null;
  async function run() {
    if (stopped || document.hidden || active) return;
    const controller = new AbortController();
    active = controller;
    try {
      await load(controller.signal);
    } catch {
      /* The caller owns user-facing errors. */
    } finally {
      if (active === controller) active = null;
    }
  }
  const visible = () => {
    if (document.hidden) active?.abort();
    else void run();
  };
  const timer = setInterval(() => void run(), 10000);
  document.addEventListener("visibilitychange", visible);
  void run();
  return () => {
    stopped = true;
    clearInterval(timer);
    document.removeEventListener("visibilitychange", visible);
    active?.abort();
  };
}
