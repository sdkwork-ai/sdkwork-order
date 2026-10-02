import { resolveViteEnvironment, resolveViteRuntimeProfile } from '../../../sdkwork-specs/tools/vite-runtime-profile.mjs';
import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';

import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { defineConfig, loadEnv } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { createSdkworkCredentialEntryBootstrapVitePlugin } from "@sdkwork/iam-credential-entry/vite";

const repoRoot = path.resolve(import.meta.dirname, "../..");
const workspaceNodeModules = path.join(repoRoot, "node_modules");
const workspacePnpmStore = path.join(workspaceNodeModules, ".pnpm");
const DEFAULT_PLATFORM_GATEWAY_TARGET = "http://127.0.0.1:3900";

// Shared runtime packages MUST resolve to one store instance inside the
// browser bundle. Without the pins below, pnpm's strict layout gives
// lucide-react / react-i18next their own react peers and the production
// build ships two react copies (hooks crash with "Cannot read properties
// of null (reading 'useContext')" — the same class of bug the root vitest
// config fixes for tests).
const sharedRuntimePackages = [
  "@radix-ui/react-avatar",
  "@radix-ui/react-checkbox",
  "@radix-ui/react-context-menu",
  "@radix-ui/react-dialog",
  "@radix-ui/react-dropdown-menu",
  "@radix-ui/react-hover-card",
  "@radix-ui/react-label",
  "@radix-ui/react-menubar",
  "@radix-ui/react-popover",
  "@radix-ui/react-radio-group",
  "@radix-ui/react-scroll-area",
  "@radix-ui/react-select",
  "@radix-ui/react-separator",
  "@radix-ui/react-slider",
  "@radix-ui/react-slot",
  "@radix-ui/react-switch",
  "@radix-ui/react-tabs",
  "@radix-ui/react-tooltip",
  "class-variance-authority",
  "clsx",
  "cmdk",
  "i18next",
  "lucide-react",
  "react-i18next",
  "sonner",
  "tailwind-merge",
];

function packageStorePrefix(packageName: string): string {
  const [scope, name] = packageName.startsWith("@")
    ? packageName.split("/")
    : ["", packageName];
  return scope ? `${scope}+${name}@` : `${name}@`;
}

function resolveWorkspacePackage(packageName: string): string {
  const directPath = path.join(workspaceNodeModules, packageName);
  if (existsSync(directPath)) {
    return directPath;
  }

  if (!existsSync(workspacePnpmStore)) {
    return packageName;
  }

  const pnpmEntry = readdirSync(workspacePnpmStore)
    .filter((entry) => {
      const packagePath = path.join(workspacePnpmStore, entry, "node_modules", packageName);
      return entry.startsWith(packageStorePrefix(packageName)) || existsSync(packagePath);
    })
    .sort()
    .at(-1);

  if (!pnpmEntry) {
    return packageName;
  }

  return path.join(workspacePnpmStore, pnpmEntry, "node_modules", packageName);
}

function escapeRegExp(pattern: string): string {
  return pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

type TsconfigAliasEntry = {
  find: string | RegExp;
  replacement: string;
};

function loadTsconfigAliases(): TsconfigAliasEntry[] {
  const tsconfigBasePath = path.join(repoRoot, "tsconfig.base.json");
  const tsconfigBase = JSON.parse(readFileSync(tsconfigBasePath, "utf8"));
  const pathMappings = tsconfigBase?.compilerOptions?.paths ?? {};
  const runtimeAliases = new Set([
    "react",
    "react-dom",
    "react/jsx-runtime",
    "react/jsx-dev-runtime",
  ]);

  const entries: TsconfigAliasEntry[] = [];
  for (const [find, replacements] of Object.entries(pathMappings)) {
    if (runtimeAliases.has(find)) {
      continue;
    }

    const replacement = Array.isArray(replacements) ? replacements[0] : undefined;
    if (typeof replacement !== "string") {
      continue;
    }

    const resolvedReplacement = path.resolve(
      repoRoot,
      replacement.endsWith("/*") ? replacement.slice(0, -2) : replacement,
    );

    // Exact mappings (e.g. "@sdkwork/order-h5-shell") must match only the
    // bare package name so subpath imports resolve through package.json
    // exports; wildcard mappings stay prefix matches.
    if (find.endsWith("/*")) {
      entries.push({
        find: find.slice(0, -2),
        replacement: resolvedReplacement,
      });
    } else {
      entries.push({
        find: new RegExp(`^${escapeRegExp(find)}$`),
        replacement: resolvedReplacement,
      });
    }
  }

  return entries.sort((left, right) => {
    const leftLen = typeof left.find === "string" ? left.find.length : left.find.source.length;
    const rightLen = typeof right.find === "string" ? right.find.length : right.find.source.length;
    return rightLen - leftLen;
  });
}

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
    optimizeDeps: {
      // Workspace-linked sources must not be frozen into the esbuild prebundle
      // (same rationale as the mall H5 root).
      exclude: [
        "@sdkwork/auth-pc-react",
        "@sdkwork/auth-runtime-pc-react",
        "@sdkwork/iam-app-sdk",
        "@sdkwork/iam-contracts",
        "@sdkwork/iam-credential-entry",
        "@sdkwork/iam-runtime",
        "@sdkwork/membership-app-sdk",
        "@sdkwork/order-app-sdk",
        "@sdkwork/order-service",
        "@sdkwork/order-h5-core",
        "@sdkwork/order-h5-shell",
        "@sdkwork/order-h5-subscription",
        "@sdkwork/order-h5-withdraw",
        "@sdkwork/order-mobile-react-orders",
        "@sdkwork/runtime-bootstrap",
        "@sdkwork/sdk-common",
        "@sdkwork/ui-mobile-react",
        "@sdkwork/utils",
      ],
    },
    resolve: {
      alias: [
        {
          find: "react",
          replacement: path.join(workspaceNodeModules, "react"),
        },
        {
          find: "react-dom",
          replacement: path.join(workspaceNodeModules, "react-dom"),
        },
        ...sharedRuntimePackages.map((packageName) => ({
          find: packageName,
          replacement: resolveWorkspacePackage(packageName),
        })),
        ...loadTsconfigAliases(),
      ],
      dedupe: [
        "react",
        "react-dom",
        "react-router",
        "react-router-dom",
        ...sharedRuntimePackages,
      ],
    },
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
