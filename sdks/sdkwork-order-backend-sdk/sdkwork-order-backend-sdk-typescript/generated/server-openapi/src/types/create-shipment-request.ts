export interface CreateShipmentRequest {
  fulfillmentId: string;
  carrierCode: string;
  trackingNo?: string;
}
