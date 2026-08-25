import type { ShopReadinessItem } from './shop-readiness-item';
export interface ShopReadiness {
    id: string;
    tenantId: string;
    organizationId: string;
    shopId: string;
    readinessScope: string;
    readinessStatus: string;
    blockingCount: number;
    warningCount: number;
    checklist: ShopReadinessItem[];
    evaluatedAt: string;
    createdAt: string;
    updatedAt: string;
    version: number;
}
//# sourceMappingURL=shop-readiness.d.ts.map