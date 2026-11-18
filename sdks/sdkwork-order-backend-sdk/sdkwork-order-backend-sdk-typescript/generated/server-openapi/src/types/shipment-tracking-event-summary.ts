export interface ShipmentTrackingEventSummary {
  eventId: string;
  trackingEventNo: string;
  eventType: string;
  eventStatus?: string;
  eventTime: string;
  locationText?: string;
}
