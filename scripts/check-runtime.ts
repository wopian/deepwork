if (typeof Bun === "undefined" || Bun.version !== "1.4.0")
  throw new Error("Bun 1.4.0 required");
console.log(`Runtime verified: Bun ${Bun.version}`);
