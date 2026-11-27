export interface OrderClassification {
  orderId: string;
  scene: 'web' | 'h5' | 'app' | 'mini_program' | 'cashier' | 'membership' | 'recharge' | 'platform' | 'default';
  tags: string[];
  remark?: string;
  updatedAt: string;
}
