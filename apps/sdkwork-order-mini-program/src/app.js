// SDKWork Order WeChat mini-program entry (domain commerce, capability order).
// Bootstraps the runtime environment and exposes it through globalData. All
// network traffic funnels through services/transport.js (single transport
// seam, enforced by tests/mp-standard-architecture.test.mjs).
let runtimeEnv = {};
try {
  runtimeEnv = require("./runtime-env.json");
} catch (error) {
  runtimeEnv = {};
}

App({
  onLaunch() {
    const injected = globalThis.__SDKWORK_ORDER_MP_RUNTIME_ENV__ ?? {};
    this.globalData = {
      environment: runtimeEnv.environment ?? injected.environment ?? "development",
      deploymentProfile: runtimeEnv.deploymentProfile ?? injected.deploymentProfile ?? "standalone",
      profileId: runtimeEnv.profileId ?? injected.profileId ?? "standalone.development",
      runtimeTarget: runtimeEnv.runtimeTarget ?? injected.runtimeTarget ?? "mini-program",
      orderApiBaseUrl:
        injected.appApiBaseUrl ?? runtimeEnv.appApiBaseUrl ?? "https://api-dev.sdkwork.com/app/v3/api",
    };
  },

  globalData: {
    environment: "development",
    deploymentProfile: "standalone",
    profileId: "standalone.development",
    runtimeTarget: "mini-program",
    orderApiBaseUrl: "https://api-dev.sdkwork.com/app/v3/api",
  },
});
