import type { OrderSceneFacet } from './order-scene-facet';
import type { OrderTagFacet } from './order-tag-facet';

export interface OrderFacet {
  scenes: OrderSceneFacet[];
  tags: OrderTagFacet[];
  totalOrders: string;
}
