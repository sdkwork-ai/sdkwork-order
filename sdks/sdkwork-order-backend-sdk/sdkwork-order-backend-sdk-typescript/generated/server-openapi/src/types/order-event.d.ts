export interface OrderEvent {
    id: string;
    eventType: string;
    fromStatus?: string;
    toStatus: string;
    actorType: string;
    actorId?: string;
    message?: string;
    createdAt: string;
}
//# sourceMappingURL=order-event.d.ts.map