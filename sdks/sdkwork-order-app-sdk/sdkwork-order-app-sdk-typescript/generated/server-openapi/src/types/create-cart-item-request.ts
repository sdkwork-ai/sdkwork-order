/** Product id and shop id are not accepted: both are attribution and the server derives the product id from the SKU row. */
export interface CreateCartItemRequest {
  skuId: string;
  quantity?: number;
}
