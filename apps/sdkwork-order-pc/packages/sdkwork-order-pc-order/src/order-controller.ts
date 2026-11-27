import {
  useMemo,
  useSyncExternalStore,
} from "react";
import {
  createSdkworkOrderMessages,
  type SdkworkOrderMessagesOverrides,
} from "./order-copy";
import {
  createSdkworkOrderService,
  type SdkworkOrderCancelInput,
  type SdkworkOrderCancelResult,
  type SdkworkOrderDashboardData,
  type SdkworkOrderDashboardQuery,
  type SdkworkOrderDetail,
  type SdkworkOrderPaymentInput,
  type SdkworkOrderPaymentResult,
  type SdkworkOrderService,
  type SdkworkOrderStatus,
  type SdkworkOrderSummary,
} from "./order-service";
import type {
  SdkworkOrderScene,
  SdkworkOrderTagMatchMode,
} from "@sdkwork/order-service";

export type SdkworkOrderFilter = "all" | SdkworkOrderStatus;

/** Scene/tag facets selected on the buyer order centre. */
export interface SdkworkOrderFacetSelection {
  /** Selected scene, or `undefined` for "all scenes". */
  scene?: SdkworkOrderScene;
  /** Selected canonical tags; empty for "all tags". */
  tags: string[];
  /** How `tags` combine. */
  tagMatch: SdkworkOrderTagMatchMode;
}

export const SDKWORK_ORDER_EMPTY_FACETS: SdkworkOrderFacetSelection = {
  tags: [],
  tagMatch: "any",
};

export interface SdkworkOrderControllerState {
  activeFilter: SdkworkOrderFilter;
  currentPage: number;
  dashboard: SdkworkOrderDashboardData;
  detail?: SdkworkOrderDetail;
  /** Currently applied scene/tag facets. */
  facets: SdkworkOrderFacetSelection;
  isBootstrapped: boolean;
  isDetailLoading: boolean;
  isDetailOpen: boolean;
  isLoading: boolean;
  isMutating: boolean;
  lastError?: string;
  pageSize: number;
  selectedOrderId?: string;
  visibleOrders: SdkworkOrderSummary[];
}

export interface SdkworkOrderController {
  bootstrap(): Promise<SdkworkOrderControllerState>;
  cancelOrder(input: SdkworkOrderCancelInput): Promise<SdkworkOrderCancelResult>;
  closeDetail(): void;
  getState(): SdkworkOrderControllerState;
  openDetail(orderId: string): Promise<SdkworkOrderControllerState>;
  payOrder(input: SdkworkOrderPaymentInput): Promise<SdkworkOrderPaymentResult>;
  refresh(): Promise<SdkworkOrderControllerState>;
  service: SdkworkOrderService;
  setFacets(facets: Partial<SdkworkOrderFacetSelection>): Promise<SdkworkOrderControllerState>;
  setFilter(filter: SdkworkOrderFilter): Promise<SdkworkOrderControllerState>;
  setPage(page: number): Promise<SdkworkOrderControllerState>;
  subscribe(listener: () => void): () => void;
}

export interface CreateSdkworkOrderControllerOptions {
  initialState?: Partial<SdkworkOrderControllerState>;
  locale?: string | null;
  messages?: SdkworkOrderMessagesOverrides;
  service?: Partial<SdkworkOrderService>;
}

function deriveVisibleOrders(
  dashboard: SdkworkOrderDashboardData,
): SdkworkOrderSummary[] {
  return dashboard.orders;
}

export function createSdkworkOrderController(
  options: CreateSdkworkOrderControllerOptions = {},
): SdkworkOrderController {
  const messages = createSdkworkOrderMessages(options.locale, options.messages);
  const copy = messages.controller;
  const fallbackDashboard = (
    options.service?.getEmptyDashboard
    ?? createSdkworkOrderService({
      locale: options.locale,
      messages: options.messages,
    }).getEmptyDashboard
  )();
  const service: SdkworkOrderService = options.service
    ? {
        ...createSdkworkOrderService({
          locale: options.locale,
          messages: options.messages,
        }),
        ...options.service,
      }
    : createSdkworkOrderService({
        locale: options.locale,
        messages: options.messages,
      });
  const listeners = new Set<() => void>();
  let state: SdkworkOrderControllerState = {
    activeFilter: "all",
    currentPage: 1,
    dashboard: fallbackDashboard,
    facets: { ...SDKWORK_ORDER_EMPTY_FACETS },
    isBootstrapped: false,
    isDetailLoading: false,
    isDetailOpen: false,
    isLoading: false,
    isMutating: false,
    pageSize: 20,
    visibleOrders: [],
    ...options.initialState,
  };
  state.visibleOrders = deriveVisibleOrders(state.dashboard);

  function emit(): void {
    listeners.forEach((listener) => listener());
  }

  function setState(
    next:
      | Partial<SdkworkOrderControllerState>
      | ((currentState: SdkworkOrderControllerState) => Partial<SdkworkOrderControllerState>),
  ): void {
    const partial = typeof next === "function" ? next(state) : next;
    state = {
      ...state,
      ...partial,
    };
    state.visibleOrders = deriveVisibleOrders(state.dashboard);
    emit();
  }

  function dashboardQuery(): SdkworkOrderDashboardQuery {
    const filter = state.activeFilter;
    return {
      page: state.currentPage,
      pageSize: state.pageSize,
      status: filter === "all" ? "all" : filter,
      ...(state.facets.scene ? { scene: state.facets.scene } : {}),
      ...(state.facets.tags.length > 0
        ? { tags: state.facets.tags, tagMatch: state.facets.tagMatch }
        : {}),
    };
  }

  async function loadDashboard(): Promise<SdkworkOrderDashboardData> {
    return service.getDashboard(dashboardQuery());
  }

  async function reload(): Promise<SdkworkOrderControllerState> {
    setState({ isLoading: true, lastError: undefined });
    try {
      const dashboard = await loadDashboard();
      setState({
        dashboard,
        isBootstrapped: true,
        isLoading: false,
      });
      return state;
    } catch (error) {
      const message = error instanceof Error ? error.message : copy.bootstrapFailed;
      setState({ isLoading: false, lastError: message });
      return state;
    }
  }

  return {
    async bootstrap() {
      // Controller owns the error surface — UI reads `state.lastError`.
      // Returning the state instead of re-throwing prevents unhandled
      // promise rejections when callers use `void controller.bootstrap()`.
      return reload();
    },

    async cancelOrder(input) {
      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const result = await service.cancelOrder(input);
        const dashboard = await loadDashboard();
        setState({
          dashboard,
          isBootstrapped: true,
          isMutating: false,
        });
        return result;
      } catch (error) {
        const message = error instanceof Error ? error.message : copy.cancelFailed;
        setState({
          isMutating: false,
          lastError: message,
        });
        return {
          cancelled: true,
          orderId: input.orderId,
          success: false,
          message,
        } satisfies SdkworkOrderCancelResult;
      }
    },

    closeDetail() {
      setState({
        detail: undefined,
        isDetailOpen: false,
        selectedOrderId: undefined,
      });
    },

    getState() {
      return state;
    },

    async openDetail(orderId) {
      setState({
        isDetailLoading: true,
        isDetailOpen: true,
        lastError: undefined,
        selectedOrderId: orderId,
      });

      try {
        const detail = await service.getOrderDetail(orderId);
        setState({
          detail,
          isDetailLoading: false,
          isDetailOpen: true,
          selectedOrderId: orderId,
        });
        return state;
      } catch (error) {
        const message = error instanceof Error ? error.message : copy.detailFailed;
        setState({
          isDetailLoading: false,
          isDetailOpen: false,
          lastError: message,
        });
        return state;
      }
    },

    async payOrder(input) {
      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const result = await service.payOrder(input);
        const dashboard = await loadDashboard();
        setState({
          dashboard,
          isBootstrapped: true,
          isMutating: false,
        });
        return result;
      } catch (error) {
        const message = error instanceof Error ? error.message : copy.payFailed;
        setState({
          isMutating: false,
          lastError: message,
        });
        return {
          amountCny: null,
          orderId: input.orderId,
          paymentParams: {},
          success: false,
          message,
        } satisfies SdkworkOrderPaymentResult;
      }
    },

    async refresh() {
      return reload();
    },

    service,

    async setFacets(facets) {
      // Merging rather than replacing lets a caller flip one facet (say the
      // scene) without re-sending the tag selection it did not touch. Facets
      // change the row set, so the page resets to 1 and the dashboard reloads.
      setState({
        currentPage: 1,
        facets: {
          scene: facets.scene === undefined ? state.facets.scene : facets.scene || undefined,
          tags: facets.tags === undefined ? state.facets.tags : [...facets.tags],
          tagMatch: facets.tagMatch ?? state.facets.tagMatch,
        },
      });
      return reload();
    },

    async setFilter(filter) {
      if (filter === state.activeFilter) {
        return state;
      }
      // Switching the filter changes the underlying row set, so reset to the
      // first page to keep the visible page aligned with the new filter scope.
      // The dashboard is reloaded immediately so the server returns the
      // filtered subset with correct pagination metadata.
      setState({
        activeFilter: filter,
        currentPage: 1,
      });
      return reload();
    },

    async setPage(page) {
      const nextPage = Math.max(1, Math.floor(page));
      if (nextPage === state.currentPage) {
        return state;
      }
      setState({ currentPage: nextPage });
      return reload();
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

export function useSdkworkOrderController(
  controller?: SdkworkOrderController,
  options?: Pick<CreateSdkworkOrderControllerOptions, "locale" | "messages" | "service">,
): SdkworkOrderController {
  return useMemo(
    () => controller ?? createSdkworkOrderController({
      ...(options?.locale ? { locale: options.locale } : {}),
      ...(options?.messages ? { messages: options.messages } : {}),
      ...(options?.service ? { service: options.service } : {}),
    }),
    [controller, options?.locale, options?.messages, options?.service],
  );
}

export function useSdkworkOrderControllerState(
  controller: SdkworkOrderController,
): SdkworkOrderControllerState {
  return useSyncExternalStore(
    controller.subscribe,
    controller.getState,
    controller.getState,
  );
}
