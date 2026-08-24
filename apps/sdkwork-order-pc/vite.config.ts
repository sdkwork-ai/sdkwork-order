import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';
function resolveViteEnvironment(mode, processEnv = process.env) {
  const profileMatch = /^(standalone|cloud)\.(development|test|staging|production)$/u.exec(mode ?? '');
  return profileMatch?.[2]
    ?? (['development', 'test', 'staging', 'production'].includes(processEnv.SDKWORK_ENVIRONMENT ?? '')
      ? processEnv.SDKWORK_ENVIRONMENT
      : 'production');
}
import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig, loadEnv } from "vite";

const repoRoot = path.resolve(import.meta.dirname, "../..");
const orderAppSdkEntry = path.resolve(
  repoRoot,
  "sdks/sdkwork-order-app-sdk/sdkwork-order-app-sdk-typescript/src/index.ts",
);
const orderBackendSdkEntry = path.resolve(
  repoRoot,
  "sdks/sdkwork-order-backend-sdk/sdkwork-order-backend-sdk-typescript/src/index.ts",
);
const sdkCommonEntry = path.resolve(
  repoRoot,
  "../sdkwork-sdk-commons/sdkwork-sdk-common-typescript/src/index.ts",
);

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, __dirname, "");

  return {
    build: {
      outDir: resolveBrowserDistOutDir(resolveViteEnvironment(mode, process.env)),
      emptyOutDir: true,
    },
    define: {
      "process.env.SDKWORK_ACCESS_TOKEN": JSON.stringify(env.SDKWORK_ACCESS_TOKEN ?? ""),
    },
    plugins: [react()],
    resolve: {
      alias: [
        { find: "@sdkwork/sdk-common", replacement: sdkCommonEntry },
        { find: "@sdkwork/order-app-sdk", replacement: orderAppSdkEntry },
        { find: "@sdkwork/order-backend-sdk", replacement: orderBackendSdkEntry },
        {
          find: "@sdkwork/order-contracts",
          replacement: path.resolve(
            repoRoot,
            "apps/sdkwork-order-common/packages/sdkwork-order-contracts/src/index.ts",
          ),
        },
        {
          find: "@sdkwork/order-service",
          replacement: path.resolve(
            repoRoot,
            "apps/sdkwork-order-common/packages/sdkwork-order-service/src/index.ts",
          ),
        },
      ],
    },
    server: {
      port: 5181,
      host: "127.0.0.1",
      fs: {
        allow: [repoRoot],
      },
    },
  };
});
