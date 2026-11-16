/**
 * Host-injected logistics capability.
 *
 * The order surface owns the tracking UI, but shipment records come from the
 * host's composed order data plane (the brick's own client exposes the detail
 * card path only). A host wires this port once at bootstrap; before that the
 * page fails closed with a capability error instead of rendering a fake empty
 * timeline.
 */
export interface OrderLogisticsPackage {
  id: string;
  name?: string;
}

export interface OrderLogisticsEvent {
  description: string;
  occurredAt?: string;
  status?: string;
}

export interface OrderLogisticsShipment {
  shipmentId: string;
  shipmentNo?: string;
  carrier?: string;
  statusLabel?: string;
  packages: readonly OrderLogisticsPackage[];
  trackingEvents: readonly OrderLogisticsEvent[];
}

export interface OrderLogisticsRuntime {
  loadShipments(input: {
    orderId?: string;
    shipmentId?: string;
  }): Promise<{ orderId?: string; shipments: readonly OrderLogisticsShipment[] }>;
}

let runtime: OrderLogisticsRuntime | null = null;

/** Wires the host logistics capability for the shipment page. */
export function configureOrderLogisticsRuntime(nextRuntime: OrderLogisticsRuntime | null): void {
  runtime = nextRuntime;
}

/** Returns the injected runtime, or `null` when the host did not compose one. */
export function getOrderLogisticsRuntime(): OrderLogisticsRuntime | null {
  return runtime;
}
