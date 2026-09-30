export const APP_ORDER_METHOD_TREE = {
  memberships: {
    orders: {
      create: true,
    },
  },
  orders: {
    list: true,
    create: true,
    retrieve: true,
    payments: {
      create: true,
      webhooks: { receive: true },
    },
    cancel: true,
    events: { list: true },
    cancellations: { create: true },
    paymentSuccess: { retrieve: true },
    receipts: { create: true },
    statistics: { retrieve: true },
    status: { retrieve: true },
  },
  shipments: {
    retrieve: true,
    packages: { list: true },
    trackingEvents: { list: true },
  },
  recharges: {
    packages: { list: true },
    settings: { retrieve: true },
    orders: {
      create: true,
      retrieve: true,
      list: true,
      cancel: true,
    },
  },
  withdrawals: {
    requests: {
      create: true,
      retrieve: true,
    },
  },
} as const;

export type OrderRequestParams = Record<string, unknown>;
export type OrderSdkResponse<T> = Promise<
  T | { code?: number | string; data?: T; message?: string; msg?: string }
>;
export type OrderSdkMethod = (...args: any[]) => OrderSdkResponse<any>;

type MethodTree = {
  readonly [key: string]: true | MethodTree;
};

export type ClientFromMethodTree<TTree extends MethodTree> = {
  readonly [TKey in keyof TTree]: TTree[TKey] extends true
    ? OrderSdkMethod
    : TTree[TKey] extends MethodTree
      ? ClientFromMethodTree<TTree[TKey]>
      : never;
};

export type OrderAppSdkClient = {
  commerce: ClientFromMethodTree<typeof APP_ORDER_METHOD_TREE>;
};
