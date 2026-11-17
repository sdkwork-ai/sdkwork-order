export interface CreateShipmentTrackingEventRequest {
  eventType: string;
  eventStatus?: string;
  eventTime?: string;
  locationText?: string;
}
