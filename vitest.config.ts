import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const root = path.dirname(fileURLToPath(import.meta.url));
const uiReactRoot = path.resolve(root, "../sdkwork-ui/sdkwork-ui-pc-react");
const uiRadixRoot = path.join(uiReactRoot, "node_modules/@radix-ui");

const sharedUiRuntimePackages = [
  "@radix-ui/primitive",
  "@radix-ui/react-arrow",
  "@radix-ui/react-avatar",
  "@radix-ui/react-checkbox",
  "@radix-ui/react-collection",
  "@radix-ui/react-compose-refs",
  "@radix-ui/react-context",
  "@radix-ui/react-context-menu",
  "@radix-ui/react-dialog",
  "@radix-ui/react-dismissable-layer",
  "@radix-ui/react-dropdown-menu",
  "@radix-ui/react-focus-guards",
  "@radix-ui/react-focus-scope",
  "@radix-ui/react-hover-card",
  "@radix-ui/react-id",
  "@radix-ui/react-label",
  "@radix-ui/react-menu",
  "@radix-ui/react-menubar",
  "@radix-ui/react-popover",
  "@radix-ui/react-popper",
  "@radix-ui/react-portal",
  "@radix-ui/react-presence",
  "@radix-ui/react-primitive",
  "@radix-ui/react-radio-group",
  "@radix-ui/react-roving-focus",
  "@radix-ui/react-scroll-area",
  "@radix-ui/react-select",
  "@radix-ui/react-separator",
  "@radix-ui/react-slider",
  "@radix-ui/react-slot",
  "@radix-ui/react-switch",
  "@radix-ui/react-tabs",
  "@radix-ui/react-tooltip",
  "@radix-ui/react-use-controllable-state",
];

function resolveUiRuntimePackage(packageName: string): string {
  if (packageName.startsWith("@radix-ui/")) {
    const packageDir = path.join(uiRadixRoot, packageName.slice("@radix-ui/".length));
    if (existsSync(path.join(packageDir, "package.json"))) {
      return packageDir;
    }
  }

  const directPath = path.join(root, "node_modules", packageName);
  if (existsSync(directPath)) {
    return directPath;
  }

  return packageName;
}

function loadLocalReactAliases() {
  const reactRoot = path.join(root, "node_modules/react");
  const reactDomRoot = path.join(root, "node_modules/react-dom");
  return [
    { find: "react", replacement: reactRoot },
    { find: "react-dom", replacement: reactDomRoot },
    { find: "react/jsx-runtime", replacement: path.join(reactRoot, "jsx-runtime.js") },
    { find: "react/jsx-dev-runtime", replacement: path.join(reactRoot, "jsx-dev-runtime.js") },
  ];
}

function loadUiRuntimeAliases() {
  return sharedUiRuntimePackages.map((packageName) => ({
    find: packageName,
    replacement: resolveUiRuntimePackage(packageName),
  }));
}

function loadUiDistAliases() {
  return [
    {
      find: "@sdkwork/ui-pc-react/theme",
      replacement: path.join(uiReactRoot, "dist/theme.js"),
    },
    {
      find: "@sdkwork/ui-pc-react",
      replacement: path.join(uiReactRoot, "dist/index.js"),
    },
  ];
}

function forceUiDistPlugin() {
  return {
    name: "force-ui-pc-react-dist",
    enforce: "pre" as const,
    resolveId(source: string) {
      if (source === "@sdkwork/ui-pc-react/theme") {
        return path.join(uiReactRoot, "dist/theme.js");
      }
      if (source === "@sdkwork/ui-pc-react") {
        return path.join(uiReactRoot, "dist/index.js");
      }
      return null;
    },
  };
}

function loadTsconfigAliases() {
  const tsconfig = JSON.parse(readFileSync(path.join(root, "tsconfig.base.json"), "utf8"));
  const paths = tsconfig?.compilerOptions?.paths ?? {};
  const excluded = new Set(["@sdkwork/ui-pc-react", "@sdkwork/ui-pc-react/theme"]);

  return Object.entries(paths)
    .map(([find, replacements]) => {
      if (excluded.has(find)) {
        return null;
      }
      const replacement = Array.isArray(replacements) ? replacements[0] : undefined;
      if (typeof replacement !== "string") {
        return null;
      }
      return {
        find,
        replacement: path.resolve(root, replacement),
      };
    })
    .filter(Boolean)
    .sort((left, right) => right!.find.length - left!.find.length) as Array<{
    find: string;
    replacement: string;
  }>;
}

export default defineConfig({
  plugins: [forceUiDistPlugin(), react()],
  resolve: {
    dedupe: ["react", "react-dom", "react/jsx-runtime", "react/jsx-dev-runtime", ...sharedUiRuntimePackages],
    alias: [
      ...loadLocalReactAliases(),
      ...loadUiDistAliases(),
      ...loadUiRuntimeAliases(),
      ...loadTsconfigAliases(),
      // The testing library is a catalog dependency of sibling workspaces
      // (sdkwork-cloudrouter) whose installs re-link it inside this repo's
      // packages to a react-dom peer bound to a different react instance.
      // Pinning it to the repo-root instance keeps the render layer on the
      // same react as the aliased component layer.
      {
        find: /^@testing-library\/react$/,
        replacement: path.join(root, "node_modules/@testing-library/react"),
      },
      {
        find: /^@testing-library\/dom$/,
        replacement: path.join(root, "node_modules/@testing-library/dom"),
      },
      {
        find: "lucide-react",
        replacement: path.join(root, "node_modules/lucide-react"),
      },
    ],
  },
  server: {
    fs: {
      allow: [root, uiReactRoot, path.resolve(root, "../sdkwork-appbase"), path.resolve(root, "..")],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: [path.join(root, "vitest.setup.ts")],
    server: {
      deps: {
        // The order workspace is a member of sibling workspaces
        // (sdkwork-cloudrouter) whose catalogs resolve `react`, `react-dom`,
        // and the i18n/testing libraries to different pnpm store instances.
        // Externalized those import their own react copy and every hook call
        // breaks with "Cannot read properties of null". Inlining the whole
        // react ecosystem keeps every react import on the local aliased
        // instances regardless of which workspace installed last.
        inline: [
          /react/,
          /react-dom/,
          /@radix-ui\/.*/,
          /@sdkwork\/ui-pc-react/,
          /@testing-library\/react/,
        ],
      },
    },
    include: [
      "apps/sdkwork-order-pc/packages/**/*.test.ts",
      "apps/sdkwork-order-pc/packages/**/*.test.tsx",
      "apps/sdkwork-order-common/packages/**/*.test.ts",
    ],
  },
});
