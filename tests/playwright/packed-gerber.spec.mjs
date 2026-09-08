import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

const packedPhdSample = fileURLToPath(
  new URL("../../demo/zuken-cr5000-sample.phd", import.meta.url),
);

// Zuken CR-5000 style photo data: header one command per line, whole body on
// a single line, stray empty commands, and a deprecated M00 terminator.
const packedPhdSource = `%FSLAX44Y44*%
%MOMM*%
%ADD80C,0.37000*%
%ADD82R,0.18000X0.90000*%
*D80*X00100000Y00100000*D03*Y00200000*D03*X00200000Y00100000*D03*D82*X00300000Y00300000D03**X0Y0D02*M00*`;

test("Open dialog accepts Zuken .phd photo data files", async ({ page }) => {
  await page.goto("/");
  const accept = await page.locator("#file-input").getAttribute("accept");
  expect(accept.split(",")).toContain(".phd");
});

test("Gerber with many commands packed on one line loads as a layer", async ({ page }) => {
  await page.goto("/");
  await page.locator("#file-input").setInputFiles({
    name: "metal-mask-top.phd",
    mimeType: "text/plain",
    buffer: Buffer.from(packedPhdSource),
  });
  await expect(page.locator("#loading-modal")).toBeHidden({ timeout: 30_000 });

  const layer = page.locator(".gerber-layer-item");
  await expect(layer).toHaveCount(1);
  await expect(layer).toContainText("metal-mask-top.phd");
  // Circles (r 0.185) at (10,10) (10,20) (20,10) and a 0.18 x 0.90 rectangle at (30,30):
  // X 9.815..30.09, Y 9.815..30.45.
  await expect(layer).toContainText("20.275 x 20.635 mm");
  await expect(page.locator("#visible-layer-count")).toHaveText("1 / 1");
});

test("bundled Zuken CR-5000 .phd sample renders with its macro and drawn outline", async ({ page }) => {
  await page.goto("/");
  await page.locator("#file-input").setInputFiles(packedPhdSample);
  await expect(page.locator("#loading-modal")).toBeHidden({ timeout: 30_000 });

  const layer = page.locator(".gerber-layer-item");
  await expect(layer).toHaveCount(1);
  await expect(layer).toContainText("zuken-cr5000-sample.phd");
  // Drawn D10 frame (0.15 mm wide) from (12,12) to (68,48) bounds the image.
  await expect(layer).toContainText("56.150 x 36.150 mm");
  await expect(page.locator("#visible-layer-count")).toHaveText("1 / 1");
});
