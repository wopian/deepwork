import { reactive, watch } from "vue";
const defaults = {
  reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
  audio: false,
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
export function purchaseSound() {
  if (!preferences.audio) return;
  context ??= new AudioContext();
  void context.resume();
  const oscillator = context.createOscillator();
  const gain = context.createGain();
  oscillator.type = "square";
  oscillator.frequency.setValueAtTime(440, context.currentTime);
  oscillator.frequency.setValueAtTime(660, context.currentTime + 0.07);
  gain.gain.setValueAtTime(
    Math.max(0, Math.min(0.3, preferences.volume)) * 0.12,
    context.currentTime,
  );
  gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.15);
  oscillator.connect(gain);
  gain.connect(context.destination);
  oscillator.start();
  oscillator.stop(context.currentTime + 0.16);
}
