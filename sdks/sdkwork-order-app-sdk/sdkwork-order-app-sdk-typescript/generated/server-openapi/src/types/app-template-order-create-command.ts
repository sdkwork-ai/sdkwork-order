export interface AppTemplateOrderCreateCommand {
  /** deploy_app_template.uuid of a published PUBLIC listing in the caller's tenant. */
  templateUuid: string;
  /** Payment method for a paid listing; ignored for a free one. */
  paymentMethod?: string;
  /** QR payment product. H5 returns the order-bound cashierUrl; native products create a provider payment intent. */
  paymentProduct?: 'mobile_cashier_h5' | 'wechat_native' | 'alipay_native';
  clientRequestNo?: string;
  source?: string;
}
