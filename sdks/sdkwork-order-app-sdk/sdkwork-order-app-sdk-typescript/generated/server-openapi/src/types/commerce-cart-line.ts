export interface CommerceCartLine {
  /** Cart line id (the `cartItemId` checkout accepts). */
  id: string;
  skuId: string;
  spuId?: string;
  quantity: number;
  selected: boolean;
  /** False when the SKU was unpublished, archived, or deleted after the add. */
  available: boolean;
  title: string;
  skuName?: string;
  imageUrl?: string;
  /** Snapshot unit price in minor units (decimal string). */
  unitPrice: string;
  priceScale: number;
  currencyCode: string;
  /** Line total in minor units (decimal string). */
  lineTotal: string;
  /** Live catalog unit price in minor units, when the SKU still resolves. */
  currentUnitPrice?: string;
  /** True when the live catalog price differs from the captured snapshot. */
  priceChanged: boolean;
  /** Sellable units across active stock rows. Absent when the SKU tracks no inventory. */
  availableStock?: number;
  /** True when the line quantity exceeds the sellable stock; checkout enforces it. */
  stockShortage: boolean;
}
