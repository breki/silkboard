import { expect, test } from "vitest";
import { fixturePort } from "../e2e/ports.mjs";
test("fixture blocks preserve all eight relative ports", () => {
  expect(fixturePort(38124, "38940")).toBe(38940);
  expect(fixturePort(38131, "38940")).toBe(38947);
  expect(fixturePort(38124, "38124")).toBe(38124);
});
test.each(["80", "0", "-1", "65529", "038124", "abc"])(
  "rejects invalid fixture base %s",
  (base) => expect(() => fixturePort(38124, base)).toThrow(),
);
test.each([38123, 38132, 38124.5])(
  "rejects a port outside the fixture block %s",
  (value) => expect(() => fixturePort(value, "38940")).toThrow(),
);
