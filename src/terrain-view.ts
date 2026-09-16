import { Container, Sprite, Texture } from "pixi.js";
import { CELL_PIXEL, CHUNK, CHUNKS_ACROSS } from "./geometry";
import type { Game } from "./game";
import { terrainPixels } from "./terrain-pixels";
/** Only visible chunks get GPU textures. Hidden minerals never enter this class. */
export class TerrainView {
  readonly layer = new Container();
  private cache = new Map<
    number,
    { sprite: Sprite; mask?: number[]; visible?: number[] }
  >();
  update(terrain: Game["terrain"] | undefined, first: number, last: number) {
    const keep = new Set<number>();
    for (
      let cy = Math.floor(first / CHUNK);
      cy <= Math.floor(last / CHUNK);
      cy++
    ) {
      for (let cx = 0; cx < CHUNKS_ACROSS; cx++) {
        const id = cy * CHUNKS_ACROSS + cx;
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
    }
    for (const [id, entry] of this.cache)
      if (!keep.has(id)) {
        entry.sprite.destroy({ texture: true, textureSource: true });
        this.cache.delete(id);
      }
  }
  clear() {
    for (const entry of this.cache.values())
      entry.sprite.destroy({ texture: true, textureSource: true });
    this.cache.clear();
  }
}
