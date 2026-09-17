import { reactive, watch } from "vue";
const defaults = {
  surveyOverlay: false,
  reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
  audio: false,
  ambience: true,
  volume: 0.2,
  uiScale: 1,
  quality: "auto",
  numbers: "compact",
};
let saved = {};
try {
  saved = JSON.parse(localStorage.getItem("deepwork-preferences") ?? "{}");
} catch {}
export const preferences = reactive({ ...defaults, ...saved });
watch(
  preferences,
  (p) => localStorage.setItem("deepwork-preferences", JSON.stringify(p)),
  { deep: true },
);
let context: AudioContext | undefined;
let hum: GainNode | undefined;
let lastTone = -Infinity;
let lastMix = -Infinity;
let workLevel = 0;
function audioContext() {
  context ??= new AudioContext();
  if (!hum) {
    hum = context.createGain();
    hum.gain.value = 0;
    hum.connect(context.destination);
    for (const frequency of [55, 82.5]) {
      const voice = context.createOscillator();
      voice.type = "triangle";
      voice.frequency.value = frequency;
      voice.connect(hum);
      voice.start();
    }
  }
  return context;
}
function mix() {
  if (!preferences.audio && !context) return;
  const c = audioContext();
  if (preferences.audio && !document.hidden) void c.resume().catch(() => {});
  const level =
    preferences.audio && preferences.ambience && !document.hidden
      ? Math.max(0, Math.min(1, preferences.volume)) *
        (0.006 + workLevel * 0.01)
      : 0;
  hum!.gain.setTargetAtTime(level, c.currentTime, 0.15);
}
watch(() => [preferences.audio, preferences.ambience, preferences.volume], mix);
document.addEventListener("visibilitychange", mix);
/** One shared machine mix; never create one audio source per worker. */
export function productionAudio(rate: number) {
  if (performance.now() - lastMix < 250) return;
  lastMix = performance.now();
  workLevel = Math.min(1, Math.max(0, rate / 10));
  mix();
}

export function purchaseSound() {
  if (!preferences.audio) return;
  if (performance.now() - lastTone < 120) return;
  lastTone = performance.now();
  context = audioContext();
  void context.resume().catch(() => {});
  const oscillator = context.createOscillator();
  const gain = context.createGain();
  oscillator.type = "square";
  oscillator.frequency.setValueAtTime(440, context.currentTime);
  oscillator.frequency.setValueAtTime(660, context.currentTime + 0.07);
  gain.gain.setValueAtTime(
    Math.max(0, Math.min(1, preferences.volume)) * 0.04,
    context.currentTime,
  );
  gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.15);
  oscillator.connect(gain);
  gain.connect(context.destination);
  oscillator.start();
  oscillator.stop(context.currentTime + 0.16);
}
