import { vi } from "vitest";
export function mockDialogs() {
  for (const method of ["showModal", "close"]) {
    if (!HTMLDialogElement.prototype[method])
      Object.defineProperty(HTMLDialogElement.prototype, method, {
        configurable: true,
        writable: true,
        value() {},
      });
  }
  vi.spyOn(HTMLDialogElement.prototype, "showModal").mockImplementation(
    function () {
      this.open = true;
    },
  );
  vi.spyOn(HTMLDialogElement.prototype, "close").mockImplementation(
    function () {
      this.open = false;
    },
  );
}
