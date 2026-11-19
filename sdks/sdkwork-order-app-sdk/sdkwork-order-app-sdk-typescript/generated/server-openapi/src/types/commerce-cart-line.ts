export interface CommerceCartLine {
  /** Cart line id (the `cartItemId` checkout accepts). */
  id: string;
  skuId: string;
  spuId?: string;
  shopId?: string;
  quantity: number;
  title: string;
  skuName?: string;
  imageUrl?: string;
  /** Snapshot unit price in minor units (decimal string). */
  unitPrice: string;
  priceScale: number;
  currencyCode: string;
  /** Line total in minor units (decimal string). */
  lineTotal: string;
}
