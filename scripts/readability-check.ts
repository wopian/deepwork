import type { Page } from "playwright-core";

/** Computed DOM checks complement screenshots; canvas art is outside text contrast scope. */
export async function checkReadability(page: Page, screen: string) {
  // Evaluate settled colours after the 150 ms hover/selection transition.
  await page.waitForTimeout(180);
  const result = await page.evaluate(() => {
    const issues: string[] = [];
    let texts = 0,
      controls = 0;
    const rgba = (value: string) => {
      const n = value.match(/[\d.]+/g)?.map(Number) ?? [];
      return [n[0] ?? 0, n[1] ?? 0, n[2] ?? 0, n[3] ?? 1];
    };
    const blend = (front: number[], back: number[]) =>
      front
        .slice(0, 3)
        .map((v, i) => v * front[3]! + back[i]! * (1 - front[3]!));
    const luminance = (rgb: number[]) =>
      rgb.slice(0, 3).reduce((sum, v, i) => {
        const c = v / 255;
        return (
          sum +
          [0.2126, 0.7152, 0.0722][i]! *
            (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)
        );
      }, 0);
    const visible = (el: Element) => {
      const box = el.getBoundingClientRect(),
        css = getComputedStyle(el);
      return (
        box.width > 0 &&
        box.height > 0 &&
        box.bottom > 0 &&
        box.top < innerHeight &&
        box.right > 0 &&
        box.left < innerWidth &&
        css.visibility !== "hidden" &&
        css.display !== "none"
      );
    };
    for (const el of document.querySelectorAll<HTMLElement>("body *")) {
      if (!visible(el) || el.closest("script, style, canvas, .mineral-swatch"))
        continue;
      const css = getComputedStyle(el);
      const label =
        el.textContent?.trim().replace(/\s+/g, " ").slice(0, 65) ?? el.tagName;
      if (
        el.matches(
          "button, select, input:not([type=checkbox]):not([type=range]):not([type=file])",
        )
      ) {
        controls++;
        const rect = el.getBoundingClientRect();
        if (rect.width < 43.9 || rect.height < 43.9)
          issues.push(
            `Target ${rect.width.toFixed(1)}×${rect.height.toFixed(1)}: ${label}`,
          );
        if (parseFloat(css.fontSize) < 16)
          issues.push(`Control text ${css.fontSize}: ${label}`);
      }
      if (
        ![...el.childNodes].some(
          (n) => n.nodeType === Node.TEXT_NODE && n.textContent?.trim(),
        )
      )
        continue;
      texts++;
      if (parseFloat(css.fontSize) < 14)
        issues.push(`Text ${css.fontSize}: ${label}`);
      const chain: Element[] = [];
      for (let node: Element | null = el; node; node = node.parentElement)
        chain.push(node);
      let background = [255, 255, 255];
      for (const node of chain.reverse())
        background = blend(
          rgba(getComputedStyle(node).backgroundColor),
          background,
        );
      const a = luminance(blend(rgba(css.color), background)),
        b = luminance(background);
      const contrast = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      const minimum = parseFloat(css.fontSize) >= 24 ? 3 : 4.5;
      if (contrast + 0.01 < minimum)
        issues.push(`Contrast ${contrast.toFixed(2)}: ${label}`);
    }
    return { texts, controls, issues };
  });
  if (result.issues.length)
    throw new Error(`${screen}:\n${result.issues.join("\n")}`);
  return { screen, ...result };
}
