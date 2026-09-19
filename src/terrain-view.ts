import { Container, Sprite, Texture } from "pixi.js";
import { CELL_PIXEL, CHUNK, chunkOrigin } from "./geometry";
import type { Game } from "./game";
import { terrainPixels } from "./terrain-pixels";
/** Only visible chunks get GPU textures. Hidden minerals never enter this class. */
export class TerrainView {
  readonly layer = new Container();
  private cache = new Map<
    number,
    { sprite: Sprite; mask?: number[]; visible?: number[] }
  >();
  update(
    terrain: Game["terrain"] | undefined,
    first: number,
    last: number,
    left: number,
    right: number,
  ) {
    const keep = new Set<number>();
    // Neutral host ground needs no GPU texture. Allocate only explored viewport
    // chunks; overview culling has a hard budget even for very large campaigns.
    const candidates = [
      ...new Set([
        ...Object.keys(terrain?.chunks ?? {}),
        ...Object.keys(terrain?.visible ?? {}),
      ]),
    ]
      .map(Number)
      .map((id) => ({ id, origin: chunkOrigin(id) }))
      .filter(
        ({ origin: [x, y] }) =>
          x + CHUNK >= left && x <= right && y + CHUNK >= first && y <= last,
      )
      .sort(
        (a, b) =>
          Math.abs(a.origin[0] - (left + right) / 2) +
          Math.abs(a.origin[1] - (first + last) / 2) -
          Math.abs(b.origin[0] - (left + right) / 2) -
          Math.abs(b.origin[1] - (first + last) / 2),
      )
      .slice(0, 768);
    for (const {
      id,
      origin: [x, y],
    } of candidates) {
      const cx = x / CHUNK,
        cy = y / CHUNK;
      keep.add(id);
      const mask = terrain?.chunks[id],
        visible = terrain?.visible[id];
      const old = this.cache.get(id);
      if (old && old.mask === mask && old.visible === visible) continue;
      const canvas = document.createElement("canvas");
      canvas.width = CHUNK;
      canvas.height = CHUNK;
      const context = canvas.getContext("2d")!;
      const pixels = context.createImageData(CHUNK, CHUNK);
      pixels.data.set(terrainPixels(mask, visible, cy));
      context.putImageData(pixels, 0, 0);
      if (old) {
        old.sprite.destroy({ texture: true, textureSource: true });
        this.cache.delete(id);
      }
      const texture = Texture.from(canvas);
      texture.source.scaleMode = "nearest";
      const sprite = new Sprite(texture);
      sprite.position.set(
        235 + cx * CHUNK * CELL_PIXEL,
        208 + cy * CHUNK * CELL_PIXEL,
      );
      sprite.scale.set(CELL_PIXEL);
      this.layer.addChild(sprite);
      this.cache.set(id, { sprite, mask, visible });
    }
    for (const [id, entry] of this.cache)
      if (!keep.has(id)) {
        entry.sprite.destroy({ texture: true, textureSource: true });
        this.cache.delete(id);
      }
  }
  get residentChunks() {
    return this.cache.size;
  }
  clear() {
    for (const entry of this.cache.values())
      entry.sprite.destroy({ texture: true, textureSource: true });
    this.cache.clear();
  }
}
