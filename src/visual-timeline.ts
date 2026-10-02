import { routePosition, type CargoLeg } from "./routes";

/** Separate channels can deliver the saved replay after a newer live update. */
export function latestWorldSnapshot<
  T extends { campaign_id: string; ticks: number },
>(saved: T, live: T | null): T {
  return live &&
    live.campaign_id === saved.campaign_id &&
    live.ticks >= saved.ticks
    ? live
    : saved;
}

/** Advance only across supplied legs. Never predict past a blocked or unknown edge. */
export function motionPosition(
  position: [number, number],
  legs: CargoLeg[],
  elapsedMilliseconds: number,
  sinceSnapshot: number,
  speed = 1,
): { point: [number, number]; mode: string } {
  let elapsed = elapsedMilliseconds;
  let remaining = Math.min(200, Math.max(0, sinceSnapshot));
  for (const leg of legs) {
    const legSpeed = leg.speed ?? speed;
    const consumed =
      legSpeed > 0
        ? Math.min(
            remaining,
            Math.max(0, leg.milliseconds - elapsed) / legSpeed,
          )
        : 0;
    elapsed += consumed * legSpeed;
    remaining -= consumed;
    if (elapsed <= leg.milliseconds) {
      if (elapsed < leg.milliseconds || remaining <= 0 || legSpeed === 0) {
        return {
          point: routePosition(
            [leg.from, leg.to],
            Math.max(0, elapsed) / Math.max(1, leg.milliseconds),
          ),
          mode: leg.mode,
        };
      }
    }
    elapsed -= leg.milliseconds;
  }
  const last = legs.at(-1);
  return { point: last?.to ?? position, mode: last?.mode ?? "waiting" };
}

export function mergeWorldUpdate<
  T extends {
    workings_offset: number;
    workings: { passages: unknown[] };
    terrain: {
      chunks: Record<string, number[]>;
      visible: Record<string, number[]>;
      revealed: Record<string, number[]>;
    };
  },
>(previous: T | null, next: T, reset: boolean): T {
  if (!previous || reset) return next;
  if (next.workings_offset > 0) {
    next.workings.passages = [
      ...previous.workings.passages.slice(0, next.workings_offset),
      ...next.workings.passages,
    ];
  }
  for (const key of ["chunks", "visible", "revealed"] as const) {
    next.terrain[key] = { ...previous.terrain[key], ...next.terrain[key] };
  }
  return next;
}
