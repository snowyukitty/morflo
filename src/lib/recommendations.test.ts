import { describe, expect, test } from "vitest";

import { demoCapabilities, missingDemoCapabilities, nativeDemoCapabilities } from "./demo";
import { imageOutcomeSettings, recommendedImageOutput } from "./recommendations";
import type { ImageSettings } from "../types";

const settings: ImageSettings = {
  outputFormat: "jpeg",
  quality: "best",
  resizeMode: "cover",
  width: 100,
  height: 100,
  background: "#FFFFFF",
  metadata: "preserve",
  animation: "ask",
};

const photo = { extension: "jpg", hasAlpha: false, width: 4032, height: 3024 };
const alpha = { extension: "png", hasAlpha: true, width: 640, height: 420 };

describe("capability-aware image recommendations", () => {
  test("keeps verified media-engine recommendations", () => {
    expect(recommendedImageOutput(alpha, demoCapabilities)).toBe("jpeg");
    expect(recommendedImageOutput(photo, demoCapabilities)).toBe("webp");
    expect(
      recommendedImageOutput({ extension: ".WEBP", hasAlpha: true }, demoCapabilities),
    ).toBe("png");
  });

  test("offers working defaults on a computer without FFmpeg", () => {
    expect(recommendedImageOutput(photo, nativeDemoCapabilities)).toBe("jpeg");
    expect(
      recommendedImageOutput({ extension: "tiff", hasAlpha: true }, nativeDemoCapabilities),
    ).toBe("png");
    expect(
      recommendedImageOutput({ extension: "avif", hasAlpha: false }, nativeDemoCapabilities),
    ).toBe("jpeg");
  });

  test("does not invent support when no suitable output is available", () => {
    expect(recommendedImageOutput(photo, missingDemoCapabilities)).toBeUndefined();
    expect(
      recommendedImageOutput(alpha, {
        ...nativeDemoCapabilities,
        outputs: [{ format: "jpeg", available: true }],
      }),
    ).toBe("jpeg");
    expect(
      recommendedImageOutput(
        { extension: "tiff", hasAlpha: true },
        { ...nativeDemoCapabilities, outputs: [{ format: "jpeg", available: true }] },
      ),
    ).toBeUndefined();
  });
});

describe("image outcomes", () => {
  test("fits a sharing photo without crop and preserves explicit safety choices", () => {
    const result = imageOutcomeSettings("share", photo, settings, nativeDemoCapabilities);
    expect(result).toMatchObject({
      outputFormat: "jpeg",
      quality: "balanced",
      resizeMode: "contain",
      width: 1920,
      height: 1920,
      metadata: "preserve",
      animation: "ask",
      background: "#FFFFFF",
    });
    expect(settings.resizeMode).toBe("cover");
  });

  test("never enlarges a small image or guesses unknown dimensions", () => {
    expect(
      imageOutcomeSettings("share", alpha, settings, nativeDemoCapabilities),
    ).toMatchObject({ resizeMode: "original", width: undefined, height: undefined });
    expect(
      imageOutcomeSettings("share", { extension: "jpg" }, settings, nativeDemoCapabilities)
        ?.resizeMode,
    ).toBe("original");
  });

  test("compresses opaque images as JPEG and protects alpha in the same batch", () => {
    expect(
      imageOutcomeSettings("smaller", photo, settings, nativeDemoCapabilities),
    ).toMatchObject({ outputFormat: "jpeg", quality: "smaller", resizeMode: "original" });
    expect(
      imageOutcomeSettings("smaller", alpha, settings, nativeDemoCapabilities)?.outputFormat,
    ).toBe("png");
    expect(
      imageOutcomeSettings("smaller", alpha, settings, demoCapabilities)?.outputFormat,
    ).toBe("webp");
  });

  test("applies transparency only where present and requires a proven encoder", () => {
    expect(
      imageOutcomeSettings("transparent", alpha, settings, nativeDemoCapabilities)
        ?.outputFormat,
    ).toBe("png");
    expect(
      imageOutcomeSettings("transparent", photo, settings, nativeDemoCapabilities),
    ).toBeUndefined();
    expect(
      imageOutcomeSettings("share", photo, settings, missingDemoCapabilities),
    ).toBeUndefined();
  });
});
