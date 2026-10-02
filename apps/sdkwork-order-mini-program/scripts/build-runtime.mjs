#!/usr/bin/env node
// Materialize the mini-program runtime environment bundle (src/runtime-env.json).
//
// Source of truth: config/mini-program/runtime-env.<deployment-profile>.<environment>.json
// (host-local real file wins; the committed `.example.json` is the fallback so a
// fresh clone builds out of the box).
//
// MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md section 10: the native WeChat env JSON
// MUST declare matching SDKWORK_ENVIRONMENT, SDKWORK_DEPLOYMENT_PROFILE,
// SDKWORK_PROFILE_ID and SDKWORK_RUNTIME_TARGET=mini-program. The build selects
// one profile explicitly and bundles one safe runtime module.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function argValue(name) {
  if (!process.argv.includes(name)) {
    return undefined;
  }
  return process.argv[process.argv.indexOf(name) + 1];
}

const ENVIRONMENT_ALIASES = {
  dev: "development",
  development: "development",
  test: "test",
  staging: "staging",
  prod: "production",
  production: "production",
};

const deploymentProfile = argValue("--deployment-profile") ?? "standalone";
const environmentToken = argValue("--environment") ?? "dev";
const environment = ENVIRONMENT_ALIASES[environmentToken];
if (!environment) {
  console.error(
    `[order-mp] unknown environment "${environmentToken}" (expected dev|test|staging|prod)`,
  );
  process.exit(1);
}

const manifest = JSON.parse(readFileSync(path.join(appRoot, "sdkwork.app.config.json"), "utf8"));
if (!manifest.environments?.[environment]) {
  console.error(`[order-mp] manifest sdkwork.app.config.json has no environment: ${environment}`);
  process.exit(1);
}
if (!manifest.runtime?.supportedDeploymentProfiles?.includes(deploymentProfile)) {
  console.error(
    `[order-mp] manifest runtime.supportedDeploymentProfiles has no "${deploymentProfile}"`,
  );
  process.exit(1);
}

const configName = `runtime-env.${deploymentProfile}.${environmentToken}.json`;
const candidates = [
  path.join(appRoot, "config", "mini-program", configName),
  path.join(appRoot, "config", "mini-program", configName.replace(/\.json$/u, ".example.json")),
];
const configPath = candidates.find((candidate) => existsSync(candidate));
if (!configPath) {
  console.error(
    `[order-mp] missing runtime env template for ${deploymentProfile}.${environmentToken}:\n` +
      candidates.map((candidate) => `  - ${candidate}`).join("\n"),
  );
  process.exit(1);
}

const template = JSON.parse(readFileSync(configPath, "utf8"));

const problems = [];
if (template.SDKWORK_ENVIRONMENT !== environment) {
  problems.push(
    `SDKWORK_ENVIRONMENT must be "${environment}" but is "${template.SDKWORK_ENVIRONMENT}"`,
  );
}
if (template.SDKWORK_DEPLOYMENT_PROFILE !== deploymentProfile) {
  problems.push(
    `SDKWORK_DEPLOYMENT_PROFILE must be "${deploymentProfile}" but is "${template.SDKWORK_DEPLOYMENT_PROFILE}"`,
  );
}
if (typeof template.SDKWORK_PROFILE_ID !== "string" || template.SDKWORK_PROFILE_ID.trim() === "") {
  problems.push("SDKWORK_PROFILE_ID must be a non-empty profile id");
}
if (template.SDKWORK_RUNTIME_TARGET !== "mini-program") {
  problems.push(
    `SDKWORK_RUNTIME_TARGET must be "mini-program" but is "${template.SDKWORK_RUNTIME_TARGET}"`,
  );
}
const baseUrl = typeof template.appApiBaseUrl === "string" ? template.appApiBaseUrl.trim() : "";
if (!/^https?:\/\//u.test(baseUrl) || !baseUrl.includes("/app/v3/api")) {
  problems.push(`appApiBaseUrl must be an http(s) /app/v3/api base URL but is "${baseUrl}"`);
}
if (problems.length > 0) {
  console.error(`[order-mp] invalid runtime env template ${path.basename(configPath)}:`);
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  process.exit(1);
}

const runtimeEnv = {
  appApiBaseUrl: baseUrl,
  environment,
  deploymentProfile,
  profileId: template.SDKWORK_PROFILE_ID,
  runtimeTarget: template.SDKWORK_RUNTIME_TARGET,
};

mkdirSync(path.join(appRoot, "src"), { recursive: true });
writeFileSync(
  path.join(appRoot, "src", "runtime-env.json"),
  `${JSON.stringify(runtimeEnv, null, 2)}\n`,
);
console.log(
  `[order-mp] materialized src/runtime-env.json (${runtimeEnv.profileId}, from ${path.basename(configPath)})`,
);
