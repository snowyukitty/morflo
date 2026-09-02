import { describe, expect, test } from "vitest";

import { recommendedImageOutput } from "./recommendations";

describe("recommendedImageOutput", () => {
  test("keeps the high-value PNG and JPEG journeys immediate", () => {
    expect(recommendedImageOutput({ extension: "png", hasAlpha: true })).toBe("jpeg");
    expect(recommendedImageOutput({ extension: "jpg", hasAlpha: false })).toBe("webp");
  });

  test("never recommends an unexplained same-format WebP conversion", () => {
    expect(recommendedImageOutput({ extension: ".WEBP", hasAlpha: true })).toBe("png");
    expect(recommendedImageOutput({ extension: "webp", hasAlpha: false })).toBe("jpeg");
  });

  test("uses WebP as the conservative default for other image families", () => {
    expect(recommendedImageOutput({ extension: "tiff", hasAlpha: true })).toBe("webp");
    expect(recommendedImageOutput({ extension: "avif", hasAlpha: false })).toBe("webp");
  });
});
