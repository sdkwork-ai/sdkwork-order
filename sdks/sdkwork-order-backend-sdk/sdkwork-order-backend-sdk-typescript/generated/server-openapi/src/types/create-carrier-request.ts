export interface CreateCarrierRequest {
  carrierCode: string;
  carrierName: string;
  trackingUrlTemplate?: string;
  sortOrder?: number;
  status?: string;
}
