<script setup lang="ts">
defineProps<{ name: string }>();
const paths: Record<string, string> = {
  Build: "M4 20 16 8m-9-4 3-2 10 10-2 3M3 17l4 4M16 3l5 5",
  Crew: "M8 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8m8-1a3 3 0 1 0 0-6M2 21v-3a6 6 0 0 1 12 0v3m3-8a5 5 0 0 1 5 5v3",
  Industry: "M3 21V9l6 4V7l6 4V3h5v18ZM7 17h1m4 0h1m4 0h1",
  Minerals: "m12 2 8 6-3 12H7L4 8Zm-8 6h16M7 20 9 8l3-6 3 6 2 12",
  More: "M5 11h2v2H5Zm6 0h2v2h-2Zm6 0h2v2h-2Z",
  close: "m6 6 12 12M6 18 18 6",
  chevron: "m9 5 7 7-7 7",
  credits: "m12 3 9 9-9 9-9-9Zm0 5 4 4-4 4-4-4Z",
  depth: "M8 3v15m-4-4 4 4 4-4M16 4h5m-5 5h3m-3 5h5m-5 5h3",
  camera: "M8 3H3v5m13-5h5v5M3 16v5h5m8 0h5v-5M8 12h8m-4-4v8",
  plus: "M12 5v14M5 12h14",
  minus: "M5 12h14",
  surface: "m5 11 7-7 7 7M12 4v16",
  follow: "M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10M12 2v3m0 14v3M2 12h3m14 0h3",
  survey: "M9 19V7a3 3 0 0 1 6 0v12m-9 0h12M9 8h6m-6 5h6M4 5 2 3m18 2 2-2",
  check: "m4 12 5 5L20 6",
  warning: "m12 3 10 18H2ZM12 9v5m0 3v1",
  guide: "m3 4 8 2v15l-8-2Zm18 0-8 2v15l8-2Z",
  logo: "m12 2 10 6v8l-10 6-10-6V8Zm-6 7 12 7M6 15l12-6",
};
</script>
<template>
  <svg
    class="game-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] ?? paths.guide" />
  </svg>
</template>
