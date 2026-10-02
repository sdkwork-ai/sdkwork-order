export interface SdkworkOrderH5SessionSnapshot {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
  sessionId?: string;
  context?: {
    tenantId?: string;
    userId?: string;
    organizationId?: string;
    sessionId?: string;
    appId?: string;
    environment?: string;
    deploymentMode?: string;
  };
  updatedAt?: string;
}

export interface SdkworkOrderH5SessionStorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface SdkworkOrderH5SessionStore {
  clearSession(): void;
  getSnapshot(): SdkworkOrderH5SessionSnapshot;
  refreshSession(): SdkworkOrderH5SessionSnapshot;
  setSession(nextSession: SdkworkOrderH5SessionSnapshot): void;
  subscribe(listener: (snapshot: SdkworkOrderH5SessionSnapshot) => void): () => void;
}

export const SDKWORK_ORDER_H5_SESSION_STORAGE_KEY = "sdkwork-order-h5-session";

function readInitialSession(
  storage: SdkworkOrderH5SessionStorageLike | undefined,
  storageKey: string,
): SdkworkOrderH5SessionSnapshot {
  if (!storage) {
    return {};
  }

  try {
    const raw = storage.getItem(storageKey);
    return raw ? (JSON.parse(raw) as SdkworkOrderH5SessionSnapshot) : {};
  } catch {
    return {};
  }
}

export function createSdkworkOrderH5SessionStore(
  storage?: SdkworkOrderH5SessionStorageLike,
  storageKey = SDKWORK_ORDER_H5_SESSION_STORAGE_KEY,
): SdkworkOrderH5SessionStore {
  let snapshot = readInitialSession(storage, storageKey);
  const listeners = new Set<(snapshot: SdkworkOrderH5SessionSnapshot) => void>();

  const emit = () => {
    for (const listener of listeners) {
      listener(snapshot);
    }
  };

  const persist = () => {
    if (!storage) {
      return;
    }

    if (!snapshot.authToken && !snapshot.accessToken && !snapshot.refreshToken) {
      storage.removeItem(storageKey);
      return;
    }

    storage.setItem(storageKey, JSON.stringify(snapshot));
  };

  return {
    clearSession() {
      snapshot = {};
      persist();
      emit();
    },
    getSnapshot() {
      return snapshot;
    },
    refreshSession() {
      snapshot = readInitialSession(storage, storageKey);
      emit();
      return snapshot;
    },
    setSession(nextSession) {
      snapshot = {
        ...nextSession,
        updatedAt: new Date().toISOString(),
      };
      persist();
      emit();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

export function hasSdkworkOrderH5IamSession(
  snapshot: SdkworkOrderH5SessionSnapshot,
): boolean {
  return Boolean(snapshot.authToken && snapshot.accessToken && snapshot.context?.tenantId);
}

let sessionStoreLocator: (() => SdkworkOrderH5SessionStore | undefined) | undefined;

/**
 * Registers the locator the app bootstrap uses to expose the live session
 * store to feature packages. Feature packages must not import app code, so
 * they resolve the store through this locator instead.
 */
export function registerSdkworkOrderH5SessionStoreLocator(
  locator: () => SdkworkOrderH5SessionStore | undefined,
): void {
  sessionStoreLocator = locator;
}

export function getSdkworkOrderH5SessionStore(): SdkworkOrderH5SessionStore | undefined {
  return sessionStoreLocator?.();
}
