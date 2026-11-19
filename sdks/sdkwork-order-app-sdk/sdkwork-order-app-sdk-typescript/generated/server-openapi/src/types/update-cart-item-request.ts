/** At least one of quantity or selected is required. */
export interface UpdateCartItemRequest {
  quantity?: number;
  /** Whether the line participates in checkout. */
  selected?: boolean;
}
