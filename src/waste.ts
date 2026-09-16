export interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  age: number;
}
/** Visual-only particles; disposal and resource recovery belong to Rust. */
export class WasteParticles {
  readonly items: Particle[] = [];
  private seed = 17;
  constructor(public limit = 600) {}
  emit(count: number, sourceX = 1000, sourceY = 140) {
    for (
      let i = 0;
      i < Math.min(count, 30) && this.items.length < this.limit;
      i++
    ) {
      this.seed = (Math.imul(this.seed, 1664525) + 1013904223) >>> 0;
      this.items.push({
        x: sourceX,
        y: sourceY,
        vx: 1 + (this.seed % 20) / 20,
        vy: -1.4,
        age: 0,
      });
    }
  }
  step(delta: number, floor: number | ((x: number) => number)) {
    if (this.items.length > this.limit) this.items.length = this.limit;
    const dt = Math.min(2, Math.max(0, delta));
    for (let i = this.items.length - 1; i >= 0; i--) {
      const p = this.items[i];
      p.age += dt;
      p.vy += 0.12 * dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      if (
        p.y >= (typeof floor === "number" ? floor : floor(p.x)) ||
        p.age > 240
      )
        this.items.splice(i, 1);
    }
  }
}
