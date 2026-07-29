import { expect, test } from "vitest";

test("boots the jsdom test runtime", () => {
  expect(document.body).toBeDefined();
});
