import { cpSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";

const resDir = resolve("src-tauri/gen/android/app/src/main/res");
const sourceDir = resolve("src-tauri/icons/android");

if (!existsSync(resDir) || !existsSync(sourceDir)) {
  console.log("[android-icons] Android project not generated; skipping icon sync.");
  process.exit(0);
}

const entries = [
  "mipmap-mdpi",
  "mipmap-hdpi",
  "mipmap-xhdpi",
  "mipmap-xxhdpi",
  "mipmap-xxxhdpi",
  "mipmap-anydpi-v26",
  "values",
];

for (const entry of entries) {
  const source = join(sourceDir, entry);
  if (!existsSync(source)) {
    continue;
  }

  cpSync(source, join(resDir, entry), { recursive: true, force: true });
}

console.log("[android-icons] Synced wifi launcher icons into the Android project.");
