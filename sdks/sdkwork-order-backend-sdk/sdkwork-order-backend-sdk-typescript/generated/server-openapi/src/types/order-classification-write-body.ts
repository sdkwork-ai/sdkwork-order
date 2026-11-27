export interface OrderClassificationWriteBody {
  /** Tags to add/remove, or the full replacement set. Canonicalized server-side (lowercased, whitespace folded to `-`, deduplicated, rejected characters removed by validation). */
  tags: string[];
  /** Replacement scene. Ignored by add/remove endpoints. */
  scene?: 'web' | 'h5' | 'app' | 'mini_program' | 'cashier' | 'membership' | 'recharge' | 'platform' | 'default';
  remark?: string;
}
