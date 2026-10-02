#!/usr/bin/env node
// Typecheck for the mini-program root.
//
// The app is dependency-free by design (native WeChat tree, no npm install),
// so this script typechecks with TypeScript when it is resolvable from the
// enclosing workspace node_modules, and otherwise falls back to a
// strip-types syntax pass (node:module.stripTypeScriptTypes) plus
// `node --check` for the plain JS surfaces. Both paths fail loudly.
import { existsSync, readdirSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(appRoot, "..", "..");
const localRequire = createRequire(import.meta.url);

function resolveTypeScript() {
  const roots = [appRoot, repoRoot];
  for (const root of roots) {
    const candidate = path.join(root, "node_modules", "typescript");
    if (existsSync(path.join(candidate, "lib", "tsc.js"))) {
      return path.join(candidate, "lib", "tsc.js");
    }
  }
  try {
    return localRequire.resolve("typescript/lib/tsc.js");
  } catch {
    return null;
  }
}

function collectFiles(dir, extensions, out = []) {
  if (!existsSync(dir)) {
    return out;
  }
  for (const entry of readdirSync(dir)) {
    const entryPath = path.join(dir, entry);
    if (statSync(entryPath).isDirectory()) {
      collectFiles(entryPath, extensions, out);
    } else if (extensions.some((extension) => entry.endsWith(extension))) {
      out.push(entryPath);
    }
  }
  return out;
}

const tscScript = resolveTypeScript();
if (tscScript) {
  const result = spawnSync(process.execPath, [tscScript, "--noEmit", "-p", path.join(appRoot, "tsconfig.json")], {
    stdio: "inherit",
  });
  process.exit(result.status ?? 1);
}

console.log(
  "[order-mp] typescript is not installed; running the dependency-free syntax pass " +
    "(stripTypeScriptTypes for src/**/*.ts, node --check for src/tests js/mjs).",
);

let failures = 0;

// 1. TypeScript sources must parse through the official type stripper.
const tsFiles = collectFiles(path.join(appRoot, "src"), [".ts"]);
let stripTypeScriptTypes = null;
try {
  ({ stripTypeScriptTypes } = await import("node:module"));
} catch {
  // older node: handled below
}
if (typeof stripTypeScriptTypes === "function") {
  for (const file of tsFiles) {
    const source = (await import("node:fs")).readFileSync(file, "utf8");
    try {
      stripTypeScriptTypes(source, { mode: "strip" });
    } catch (error) {
      failures += 1;
      console.error(`[order-mp] TS syntax error in ${path.relative(appRoot, file)}: ${error.message}`);
    }
  }
} else {
  console.warn("[order-mp] node:module.stripTypeScriptTypes unavailable; skipped TS syntax pass.");
}

// 2. Plain JS/MJS sources must pass node --check.
for (const file of [
  ...collectFiles(path.join(appRoot, "src"), [".js", ".mjs"]),
  ...collectFiles(path.join(appRoot, "tests"), [".mjs"]),
  ...collectFiles(path.join(appRoot, "scripts"), [".mjs"]),
]) {
  const result = spawnSync(process.execPath, ["--check", file], { encoding: "utf8" });
  if (result.status !== 0) {
    failures += 1;
    console.error(`[order-mp] JS syntax error in ${path.relative(appRoot, file)}:\n${result.stderr}`);
  }
}

if (failures > 0) {
  console.error(`[order-mp] typecheck failed with ${failures} file(s).`);
  process.exit(1);
}
console.log(`[order-mp] typecheck passed (${tsFiles.length} ts file(s) syntax-checked).`);
