import {
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const isWindows = process.platform === "win32";
const unsignedApk = resolve(
  "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk",
);
const signedApk = resolve(
  "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-debugsigned.apk",
);
const debugKeystore = join(homedir(), ".android", "debug.keystore");

function fail(message) {
  console.error(`[android-sign] ${message}`);
  process.exit(1);
}

function run(command, args) {
  const needsShell = isWindows && command.toLowerCase().endsWith(".bat");
  const result = spawnSync(command, args, {
    stdio: "inherit",
    shell: needsShell,
  });

  if (result.error) {
    fail(`${command} 启动失败: ${result.error.message}`);
  }
  if (result.status !== 0) {
    fail(`${command} 退出码 ${result.status}`);
  }
}

function findAndroidSdk() {
  const sdk = process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT;
  if (!sdk || !existsSync(sdk)) {
    fail("未找到 ANDROID_HOME 或 ANDROID_SDK_ROOT。");
  }
  return sdk;
}

function findBuildTool(toolName) {
  const buildToolsDir = join(findAndroidSdk(), "build-tools");
  if (!existsSync(buildToolsDir)) {
    fail(`未找到 Android build-tools 目录: ${buildToolsDir}`);
  }

  const executable = isWindows ? `${toolName}.bat` : toolName;
  const versions = readdirSync(buildToolsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));

  for (const version of versions) {
    const candidate = join(buildToolsDir, version, executable);
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  fail(`未找到 ${toolName}。`);
}

function findKeytool() {
  const javaHome = process.env.JAVA_HOME;
  if (javaHome) {
    const candidate = join(javaHome, "bin", isWindows ? "keytool.exe" : "keytool");
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  return isWindows ? "keytool.exe" : "keytool";
}

function ensureDebugKeystore() {
  if (existsSync(debugKeystore)) {
    return;
  }

  mkdirSync(dirname(debugKeystore), { recursive: true });
  console.log(`[android-sign] Creating debug keystore: ${debugKeystore}`);
  run(findKeytool(), [
    "-genkeypair",
    "-v",
    "-keystore",
    debugKeystore,
    "-storepass",
    "android",
    "-alias",
    "androiddebugkey",
    "-keypass",
    "android",
    "-keyalg",
    "RSA",
    "-keysize",
    "2048",
    "-validity",
    "10000",
    "-dname",
    "CN=Android Debug,O=Android,C=US",
  ]);
}

if (!existsSync(unsignedApk)) {
  fail(`未找到 unsigned release APK: ${unsignedApk}`);
}

ensureDebugKeystore();

const apksigner = findBuildTool("apksigner");
rmSync(signedApk, { force: true });

run(apksigner, [
  "sign",
  "--ks",
  debugKeystore,
  "--ks-key-alias",
  "androiddebugkey",
  "--ks-pass",
  "pass:android",
  "--key-pass",
  "pass:android",
  "--out",
  signedApk,
  unsignedApk,
]);

run(apksigner, ["verify", "--verbose", signedApk]);
console.log(`[android-sign] Signed APK: ${signedApk}`);
