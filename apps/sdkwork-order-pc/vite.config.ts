import { resolveViteEnvironment, resolveLucideReactEntry } from '../../../sdkwork-specs/tools/vite-runtime-profile.mjs';
import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';

import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig, loadEnv } from "vite";
import { createSdkworkCredentialEntryBootstrapVitePlugin } from '@sdkwork/iam-credential-entry/vite';

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

  const bootstrapAccessToken = env.SDKWORK_ACCESS_TOKEN ?? process.env.SDKWORK_ACCESS_TOKEN;
  return {
    build: {
      outDir: resolveBrowserDistOutDir(resolveViteEnvironment(mode, process.env)),
      emptyOutDir: true,
    },
    plugins: [
      // The bootstrap credential reaches the renderer only through the shared IAM
      // plugin (dev-server HTML injection as
      // `globalThis.__SDKWORK_CREDENTIAL_ENTRY_BOOTSTRAP_ACCESS_TOKEN__`).
      // `define['process.env.SDKWORK_ACCESS_TOKEN']` is NOT a valid handoff
      // (IAM_CREDENTIAL_ENTRY_SPEC.md section 4/5).
      createSdkworkCredentialEntryBootstrapVitePlugin({
        accessToken: bootstrapAccessToken,
        environment: resolveViteEnvironment(mode, process.env),
      }),
      react(),
    ],
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
