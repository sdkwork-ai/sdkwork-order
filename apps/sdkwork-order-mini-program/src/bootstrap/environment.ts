/**
 * Mini-program runtime environment contract.
 *
 * Base URLs come from build-time injection via the runtime bundle
 * (scripts/build-runtime.mjs materializes src/runtime-env.json from
 * config/mini-program/runtime-env.<profile>.<env>.json), never from
 * hard-coded secrets. MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md section 10.
 */
export interface SdkworkOrderMpRuntimeEnvironment {
  /** Order app-api base, e.g. https://api-dev.sdkwork.com/app/v3/api. */
  appApiBaseUrl: string;
  /** Canonical SDKWORK_ENVIRONMENT (development|test|staging|production). */
  environment: string;
  /** Canonical SDKWORK_DEPLOYMENT_PROFILE (standalone|cloud). */
  deploymentProfile: string;
  /** SDKWORK_PROFILE_ID, e.g. "standalone.development". */
  profileId: string;
  /** Always "mini-program" for this root. */
  runtimeTarget: string;
}

export function readRuntimeEnv(): SdkworkOrderMpRuntimeEnvironment {
  const globalRuntime = (globalThis as {
    __SDKWORK_ORDER_MP_RUNTIME_ENV__?: Partial<SdkworkOrderMpRuntimeEnvironment>;
  }).__SDKWORK_ORDER_MP_RUNTIME_ENV__ ?? {};
  return {
    appApiBaseUrl: globalRuntime.appApiBaseUrl ?? "https://api-dev.sdkwork.com/app/v3/api",
    environment: globalRuntime.environment ?? "development",
    deploymentProfile: globalRuntime.deploymentProfile ?? "standalone",
    profileId: globalRuntime.profileId ?? "standalone.development",
    runtimeTarget: globalRuntime.runtimeTarget ?? "mini-program",
  };
}
