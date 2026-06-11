import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const gradleFile = resolve("src-tauri/gen/android/app/build.gradle.kts");

if (!existsSync(gradleFile)) {
  console.log("[android-size] Gradle project not generated; skipping resource shrink patch.");
  process.exit(0);
}

const source = readFileSync(gradleFile, "utf8");

if (source.includes("isShrinkResources = true")) {
  console.log("[android-size] Android resource shrink already enabled.");
  process.exit(0);
}

const next = source.replace(
  /getByName\("release"\)\s*\{\r?\n(\s*)isMinifyEnabled = true/,
  (match, indent) => `${match}\n${indent}isShrinkResources = true`,
);

if (next === source) {
  console.warn("[android-size] Release build block not found; resource shrink patch skipped.");
  process.exit(0);
}

writeFileSync(gradleFile, next);
console.log("[android-size] Enabled Android resource shrink for release builds.");
