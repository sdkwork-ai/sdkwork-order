import { resolveViteEnvironment, resolveViteRuntimeProfile } from '../../../sdkwork-specs/tools/vite-runtime-profile.mjs';
import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';

import path from "node:path";
import { defineConfig, loadEnv } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { createSdkworkCredentialEntryBootstrapVitePlugin } from "@sdkwork/iam-credential-entry/vite";

const repoRoot = path.resolve(import.meta.dirname, "../..");
const DEFAULT_PLATFORM_GATEWAY_TARGET = "http://127.0.0.1:3900";

function resolveDevProxyTarget(value: string | undefined, fallback: string): string {
  const trimmed = value?.trim();
  if (!trimmed) {
    return fallback;
  }
  if (/^https?:\/\//iu.test(trimmed)) {
    try {
      return new URL(trimmed).origin;
    } catch {
      return fallback;
    }
  }
  return fallback;
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, import.meta.dirname, "");
  const profile = resolveViteRuntimeProfile(mode, process.env);
  const bootstrapAccessToken = env.SDKWORK_ACCESS_TOKEN ?? process.env.SDKWORK_ACCESS_TOKEN;
  const platformGatewayUrl =
    env.VITE_SDKWORK_ORDER_API_ORIGIN
    ?? env.SDKWORK_API_BASE_URL
    ?? env.VITE_SDKWORK_CLOUDROUTER_PLATFORM_API_GATEWAY_HTTP_URL
    ?? env.SDKWORK_CLOUDROUTER_PLATFORM_API_GATEWAY_HTTP_URL;
  const apiProxyTarget = resolveDevProxyTarget(
    process.env.SDKWORK_ORDER_H5_DEV_APP_API_PROXY_TARGET ?? platformGatewayUrl,
    DEFAULT_PLATFORM_GATEWAY_TARGET,
  );

  return {
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
      tailwindcss(),
    ],
    build: {
      // Canonical browser dist layout (APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC
      // §2.1): dist/<deploymentProfile>/<envAlias>, never a bare dist/.
      outDir: resolveBrowserDistOutDir(profile.environment, profile.deploymentProfile),
      sourcemap: profile.environment !== "production",
      emptyOutDir: true,
    },
    server: {
      host: "127.0.0.1",
      port: 5182,
      proxy: {
        "/app/v3/api": {
          changeOrigin: true,
          target: apiProxyTarget,
        },
      },
      fs: {
        allow: [repoRoot, path.resolve(repoRoot, "..")],
      },
    },
  };
});
