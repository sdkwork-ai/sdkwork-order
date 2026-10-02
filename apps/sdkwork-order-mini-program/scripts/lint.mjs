#!/usr/bin/env node
// Dependency-free lint for the mini-program root.
//
// Checks:
//   1. every JSON file in the app root parses;
//   2. the single transport seam: `wx.request(` appears only in
//      src/services/transport.js across all src JS;
//   3. no `debugger` statements or stray `console.log` in src JS;
//   4. no forbidden/generated SDK transport imports in src;
//   5. committed config/project JSON carries no secret-shaped keys.
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const problems = [];

function collectFiles(dir, extensions, out = []) {
  if (!statSafe(dir)) {
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

function statSafe(candidate) {
  try {
    return statSync(candidate).isDirectory();
  } catch {
    return false;
  }
}

function relative(file) {
  return path.relative(appRoot, file).replaceAll("\\", "/");
}

// 1. JSON validity (root + config + src, excluding node_modules/dist).
for (const file of collectFiles(appRoot, [".json"])) {
  if (file.includes(`${path.sep}node_modules${path.sep}`) || file.includes(`${path.sep}dist${path.sep}`)) {
    continue;
  }
  try {
    JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    problems.push(`${relative(file)}: invalid JSON (${error.message})`);
  }
}

// 2. Single transport seam.
const transportPath = path.join(appRoot, "src", "services", "transport.js");
for (const file of collectFiles(path.join(appRoot, "src"), [".js"])) {
  const occurrences = (readFileSync(file, "utf8").match(/wx\.request\(/gu) ?? []).length;
  const isTransport = path.resolve(file) === path.resolve(transportPath);
  if (isTransport && occurrences !== 1) {
    problems.push(`${relative(file)}: the transport seam must issue exactly one wx.request, found ${occurrences}`);
  }
  if (!isTransport && occurrences > 0) {
    problems.push(`${relative(file)}: must go through services/transport.js, found wx.request`);
  }
}

// 3. debugger / console.log in src JS.
for (const file of collectFiles(path.join(appRoot, "src"), [".js"])) {
  const source = readFileSync(file, "utf8");
  if (/\bdebugger\b/u.test(source)) {
    problems.push(`${relative(file)}: debugger statement found`);
  }
  if (/console\.log\(/u.test(source)) {
    problems.push(`${relative(file)}: console.log found (use wx.showToast or drop the line)`);
  }
}

// 4. Forbidden generated/transport imports in src.
const forbiddenImportPatterns = [
  /domain-transport/u,
  /generated\/server-openapi/u,
  /from\s+["']@sdkwork\//u,
  /require\(\s*["']@sdkwork\//u,
];
for (const file of collectFiles(path.join(appRoot, "src"), [".js", ".ts"])) {
  const source = readFileSync(file, "utf8");
  for (const pattern of forbiddenImportPatterns) {
    if (pattern.test(source)) {
      problems.push(`${relative(file)}: forbidden transport/generated import (${pattern})`);
    }
  }
}

// 5. Secret-shaped keys in committed JSON configs.
const secretKeyPattern = /(secret|privatekey|password|accesstoken|refreshtoken|apikey|api_key)/iu;
const configFiles = [
  ...collectFiles(path.join(appRoot, "config"), [".json"]),
  path.join(appRoot, "project.config.json"),
  path.join(appRoot, "sdkwork.app.config.json"),
];
for (const file of configFiles) {
  if (!existsSafe(file)) {
    continue;
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(file, "utf8"));
  } catch {
    continue; // reported by check 1
  }
  const stack = [parsed];
  while (stack.length > 0) {
    const node = stack.pop();
    if (node === null || typeof node !== "object") {
      continue;
    }
    for (const [key, value] of Object.entries(node)) {
      if (secretKeyPattern.test(key.replaceAll(/[-_\s]/gu, ""))) {
        problems.push(`${relative(file)}: secret-shaped config key "${key}" must not be committed`);
      }
      if (typeof value === "object" && value !== null) {
        stack.push(value);
      }
    }
  }
}

function existsSafe(candidate) {
  try {
    return statSync(candidate).isFile();
  } catch {
    return false;
  }
}

if (problems.length > 0) {
  console.error(`[order-mp] lint found ${problems.length} problem(s):`);
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  process.exit(1);
}
console.log("[order-mp] lint passed (json validity, transport seam, no debug output, no forbidden imports, config secret scan).");
