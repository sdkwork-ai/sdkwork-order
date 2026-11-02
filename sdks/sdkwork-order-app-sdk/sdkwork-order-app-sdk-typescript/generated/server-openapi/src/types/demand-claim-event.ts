export interface DemandClaimEvent {
  id: string;
  claimId: string;
  eventType: string;
  fromStatus?: string;
  toStatus: string;
  actorType: 'provider' | 'publisher' | 'operator' | 'system';
  actorId?: string;
  message?: string;
  payload?: Record<string, unknown>;
  createdAt: string;
}
