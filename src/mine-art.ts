import type { Graphics } from "pixi.js";

const steel = 0x829b9b,
  steelShade = 0x415967,
  cream = 0xecd8b1,
  copper = 0xd99756;
function pixel(
  g: Graphics,
  x: number,
  y: number,
  w: number,
  h: number,
  colour: number,
) {
  g.rect(x, y, w, h).fill(colour);
}
/** Deterministic art only. Coordinates and upgrade ownership come from simulation. */
export function campArt(g: Graphics, x: number, floors: number) {
  const top = 157 - (floors - 1) * 15;
  pixel(g, x - 2, 186, 50, 3, 0x36474a);
  pixel(g, x, top, 46, 30 + (floors - 1) * 15, 0x996a48);
  pixel(g, x + 3, top + 3, 40, 27 + (floors - 1) * 15, 0xb58157);
  pixel(g, x + 41, top + 3, 5, 27 + (floors - 1) * 15, 0x76513d);
  for (let floor = 0; floor < floors; floor++) {
    const y = 158 - floor * 15;
    pixel(g, x, y + 13, 46, 2, 0x7d573f);
    for (let window = 0; window < 3; window++) {
      const wx = x + 5 + window * 14;
      pixel(g, wx - 1, y + 3, 8, 10, 0x563f33);
      pixel(g, wx, y + 4, 6, 7, 0xf0ca81);
      pixel(g, wx + 2, y + 4, 1, 7, 0x7b6a53);
      pixel(g, wx, y + 7, 6, 1, 0x7b6a53);
    }
  }
  pixel(g, x - 4, top - 5, 54, 5, steelShade);
  pixel(g, x - 3, top - 6, 52, 2, steel);
  pixel(g, x + 33, top - 13, 6, 7, 0x76513d);
  pixel(g, x + 32, top - 14, 8, 2, cream);
  pixel(g, x + 17, 171, 12, 17, 0x4b3c32);
  pixel(g, x + 18, 172, 10, 15, 0x73573e);
  pixel(g, x + 25, 179, 1, 2, copper);
  pixel(g, x + 13, 187, 20, 2, cream);
}
export function headframeArt(g: Graphics, x: number, tier: number) {
  for (const dx of [-18, 14]) {
    pixel(g, x + dx, 125, 5, 66, steelShade);
    pixel(g, x + dx, 125, 2, 66, steel);
  }
  for (let y = 135; y < 187; y += 13) {
    g.moveTo(x - 14, y)
      .lineTo(x + 14, y + 12)
      .stroke({ width: 2, color: steelShade });
    g.moveTo(x + 14, y)
      .lineTo(x - 14, y + 12)
      .stroke({ width: 1, color: steel });
  }
  pixel(g, x - 23, 121, 46, 6, steelShade);
  pixel(g, x - 24, 120, 48, 2, steel);
  g.circle(x, 116, 8).fill(0x314851).stroke({ width: 2, color: copper });
  pixel(g, x - 1, 109, 2, 15, cream);
  pixel(g, x - 7, 115, 14, 2, steel);
  pixel(g, x - 1, 124, 1, 68, copper);
  pixel(g, x - 24, 190, 48, 3, steelShade);
  for (let badge = 0; badge < Math.min(tier, 5); badge++)
    pixel(g, x - 19 + badge * 8, 125, 4, 2, copper);
}
export function plantArt(
  g: Graphics,
  x: number,
  height: number,
  kind: string,
  modules: number,
  tier: number,
  furnace: boolean,
) {
  const y = 188 - height;
  pixel(g, x - 3, 186, 58, 4, steelShade);
  pixel(g, x, y, 51, height, steelShade);
  pixel(g, x + 3, y + 3, 44, height - 3, steel);
  pixel(g, x + 5, y + 11, 40, height - 14, 0x24353d);
  pixel(g, x - 3, y - 3, 57, 4, 0x263f4c);
  pixel(g, x - 2, y - 4, 55, 1, steel);
  for (let rivet = 0; rivet < 5; rivet++)
    pixel(g, x + 5 + rivet * 9, y + 6, 2, 2, cream);
  if (kind === "heat") {
    pixel(g, x + 6, 161, 20, 24, 0x62514a);
    pixel(g, x + 8, 163, 16, 19, 0xb26d42);
    pixel(g, x + 11, 168, 10, 10, furnace ? 0xe99344 : 0x24353d);
    if (furnace) pixel(g, x + 13, 171, 6, 5, 0xffd481);
    pixel(g, x + 30, y - 22, 8, height + 20, 0x516975);
    pixel(g, x + 31, y - 22, 2, height + 20, steel);
    pixel(g, x + 28, y - 24, 12, 3, steelShade);
    for (let rung = y - 14; rung < 181; rung += 7)
      pixel(g, x + 39, rung, 5, 1, copper);
  } else if (kind === "chemical") {
    for (let tank = 0; tank < 3; tank++) {
      const tx = x + 7 + tank * 13;
      pixel(g, tx, y + 18, 10, height - 22, 0xb9c7ba);
      pixel(g, tx + 2, y + 18, 2, height - 22, cream);
      pixel(g, tx + 8, y + 18, 2, height - 22, steel);
      pixel(g, tx + 1, y + 14, 8, 4, steel);
      pixel(g, tx + 4, y + 9, 2, 5, copper);
      pixel(g, tx, y + 30, 10, 2, 0x57716d);
    }
    pixel(g, x + 10, y + 8, 28, 2, copper);
  } else {
    for (let row = 0; row < 3; row++)
      for (let column = 0; column < 3; column++) {
        pixel(g, x + 8 + column * 12, y + 14 + row * 10, 8, 7, steelShade);
        pixel(
          g,
          x + 9 + column * 12,
          y + 15 + row * 10,
          6,
          4,
          column % 2 ? 0x9cb899 : 0xe1ab63,
        );
      }
  }
  for (let module = 0; module < modules; module++)
    pixel(g, x + 7 + module * 10, y + 6, 4, 2, copper);
  for (let badge = 0; badge < tier; badge++)
    pixel(g, x + 4 + badge * 9, 190, 5, 2, copper);
}
export function workLight(g: Graphics, x: number, y: number) {
  g.rect(x - 7, y - 2, 14, 5).fill({ color: 0xffc77b, alpha: 0.045 });
  g.rect(x - 4, y - 1, 8, 3).fill({ color: 0xffc77b, alpha: 0.085 });
  pixel(g, x - 2, y - 2, 4, 1, 0x354a4c);
  pixel(g, x - 1, y - 1, 2, 1, 0xffd99a);
}
export function workerArt(
  g: Graphics,
  x: number,
  y: number,
  role: string,
  activity: string,
  phase: number,
  facing: number,
  reduced: boolean,
  low: boolean,
) {
  const coat =
    (
      {
        diggers: 0xd99756,
        engineers: 0x88b1b7,
        prospectors: 0xb8bc80,
        haulers: 0xc99c6c,
        operators: 0x86a990,
        reclaimers: 0xa29ab2,
      } as Record<string, number>
    )[role] ?? steel;
  const walking = activity === "walking" || activity === "climbing";
  const stride = walking && !reduced ? Math.floor(phase / 5) % 2 : 0;
  const side = facing < 0 ? -1 : 1;
  const hand = side > 0 ? x + 5 : x - 2;
  pixel(g, x, y - 8, 5, 2, copper);
  pixel(g, x - 1, y - 6, 7, 1, 0x735b3d);
  pixel(g, x + 1, y - 6, 3, 2, cream);
  pixel(g, x + (side > 0 ? 4 : 0), y - 7, 1, 1, 0xffe6a7);
  pixel(g, x, y - 4, 5, 3, coat);
  pixel(g, x, y - 2, 5, 1, 0x48525a);
  pixel(g, x + 2, y - 4, 1, 2, cream);
  pixel(g, x + stride, y - 1, 2, 1, 0x202f38);
  pixel(g, x + 3 - stride, y - 1, 2, 1, 0x202f38);
  if (activity === "climbing") pixel(g, hand, y - 6 + stride, 1, 2, cream);
  else if (activity === "digging" || activity === "building") {
    const swing = reduced ? 1 : Math.floor(phase / 8) % 3;
    pixel(g, hand, y - 4 - swing, 3, 1, steel);
    pixel(g, hand + 1, y - 3 - swing, 1, 2, copper);
    if (!reduced && swing === 2)
      for (let n = 0; n < (low ? 2 : 4); n++) {
        const life = (phase / 12 + n * 0.23) % 1;
        pixel(
          g,
          hand + side * life * (3 + n),
          y - 4 + life * life * 7,
          1,
          1,
          n % 2 ? copper : cream,
        );
      }
  } else if (activity === "surveying") {
    pixel(g, hand, y - 4, 2, 3, cream);
    pixel(g, hand, y - 4, 2, 1, 0x92babc);
  } else if (activity === "hauling") {
    pixel(g, hand, y - 3, 3, 2, copper);
  } else if (activity === "waiting" || activity === "blocked") {
    pixel(g, x + 6, y - 8, 1, 2, cream);
    pixel(g, x + 6, y - 5, 1, 1, cream);
  }
}
