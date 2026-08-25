import { formatMoney } from "@sdkwork/utils/money";
import { createOrderAppTransportClient, } from "./transport.js";
import { createSdkworkIdempotencyParams } from "./idempotency.js";
let sdkworkOrderAppServiceProvider = null;
let sdkworkOrderSessionTokenProvider = () => ({});
export function configureSdkworkOrderAppServiceProvider(provider) {
    sdkworkOrderAppServiceProvider = provider;
}
export function configureSdkworkOrderSessionTokenProvider(provider) {
    sdkworkOrderSessionTokenProvider = provider ?? (() => ({}));
}
export function getSdkworkOrderService() {
    if (!sdkworkOrderAppServiceProvider) {
        throw new Error("SDKWork order service provider is not configured. Call configureSdkworkOrderAppServiceProvider() from order PC bootstrap.");
    }
    return sdkworkOrderAppServiceProvider();
}
export function getSdkworkOrderSessionTokens() {
    const tokens = sdkworkOrderSessionTokenProvider();
    return {
        accessToken: normalizeSessionToken(tokens.accessToken),
        authToken: normalizeSessionToken(tokens.authToken),
        refreshToken: normalizeSessionToken(tokens.refreshToken),
    };
}
export function hasSdkworkOrderSession() {
    const tokens = getSdkworkOrderSessionTokens();
    return Boolean(normalizeSessionToken(tokens.authToken) || normalizeSessionToken(tokens.accessToken));
}
export function requireSdkworkOrderSession(message = "Authentication required") {
    if (!hasSdkworkOrderSession()) {
        throw new Error(message);
    }
}
export function createSdkworkOrderAppService(input) {
    return {
        checkout: input.appClient.checkout,
        memberships: input.appClient.memberships,
        orders: input.appClient.orders,
        recharges: input.appClient.recharges,
        withdrawals: input.appClient.withdrawals,
    };
}
export function createSdkworkPhysicalPurchaseService(options = {}) {
    const resolveAppService = () => options.appService ?? getSdkworkOrderService();
    return {
        async prepareCheckout(input) {
            requireSdkworkOrderSession();
            const items = input.items.map((item) => ({
                quantity: normalizePhysicalQuantity(item.quantity),
                skuId: requirePhysicalText("SKU", item.skuId),
            }));
            if (items.length === 0) {
                throw new Error("At least one physical SKU is required.");
            }
            if (new Set(items.map((item) => item.skuId)).size !== items.length) {
                throw new Error("Duplicate physical SKU lines are not allowed.");
            }
            const shippingAddress = normalizePhysicalShippingAddress(input.shippingAddress);
            const session = await resolveAppService().checkout.sessions.create({
                currencyCode: (input.currencyCode ?? "CNY").trim().toUpperCase(),
                items: items.map((item) => ({
                    quantity: String(item.quantity),
                    skuId: item.skuId,
                })),
                shippingAddress,
            }, createSdkworkIdempotencyParams());
            const checkoutSessionId = requirePhysicalText("checkout session id", session.checkoutSessionId);
            const quote = await resolveAppService().checkout.sessions.quotes.create(checkoutSessionId, createSdkworkIdempotencyParams());
            return {
                checkoutSessionId,
                currencyCode: quote.currencyCode,
                discountAmount: quote.discountAmount,
                originalAmount: quote.originalAmount,
                payableAmount: quote.payableAmount,
                quoteId: quote.quoteId,
                status: session.status,
            };
        },
        async placeOrder(checkoutSessionId) {
            requireSdkworkOrderSession();
            const normalizedSessionId = requirePhysicalText("checkout session id", checkoutSessionId);
            return resolveAppService().checkout.sessions.orders.create(normalizedSessionId, createSdkworkIdempotencyParams());
        },
    };
}
export function createSdkworkPointsRechargeService(options = {}) {
    const resolveAppService = () => options.appService ?? getSdkworkOrderService();
    return {
        async listPackages() {
            const response = await resolveAppService().recharges.packages.list({ page: 1, pageSize: 200 });
            const page = unwrapSdkworkOrderListPage(response, "Unable to load recharge packages.");
            return page.items.map(normalizePointsRechargePackage).filter((item) => item !== null);
        },
        async createOrder(input) {
            const packageId = String(input.packageId).trim();
            if (!packageId) {
                throw new Error("A recharge package is required.");
            }
            const packages = await this.listPackages();
            const selectedPackage = packages.find((item) => item.id === packageId);
            if (!selectedPackage) {
                throw new Error("The selected recharge package is unavailable.");
            }
            const paymentProduct = input.paymentProduct ?? "mobile_cashier_h5";
            const body = {
                amount: selectedPackage.priceAmount,
                currencyCode: selectedPackage.currencyCode,
                packageId: selectedPackage.id,
                paymentMethod: input.paymentMethod
                    ?? (paymentProduct === "alipay_native" ? "alipay" : "wechat_pay"),
                paymentProduct,
                source: input.source ?? "membership-token-plan",
                subject: "points_recharge",
                targetAsset: "points",
            };
            const params = createSdkworkIdempotencyParams();
            const response = await resolveAppService().recharges.orders.create(body, params);
            return normalizePointsRechargePayment(unwrapSdkworkOrderResource(response, "Unable to create points recharge order."));
        },
        async getOrderStatus(orderId) {
            const normalizedOrderId = orderId.trim();
            if (!normalizedOrderId) {
                throw new Error("A recharge order id is required.");
            }
            const response = await resolveAppService().recharges.orders.retrieve(normalizedOrderId);
            return normalizePointsRechargePayment(unwrapSdkworkOrderResource(response, "Unable to retrieve points recharge order."));
        },
    };
}
export function createSdkworkCouponRechargeService(options = {}) {
    const resolveAppService = () => options.appService ?? getSdkworkOrderService();
    return {
        async redeem(code) {
            requireSdkworkOrderSession();
            const couponCode = code.trim();
            if (!couponCode) {
                throw new Error("A coupon code is required.");
            }
            const params = createSdkworkIdempotencyParams();
            const response = await resolveAppService().orders.couponRedemptions.create({ couponCode }, params);
            return normalizeCouponRechargeResult(unwrapSdkworkOrderResource(response, "Unable to redeem this coupon."));
        },
    };
}
export function createSdkworkMembershipCheckoutService(options = {}) {
    const resolveAppService = () => options.appService ?? getSdkworkOrderService();
    const inFlightCheckouts = new Map();
    return {
        createCheckout(input) {
            requireSdkworkOrderSession();
            const isRecharge = input.action === "recharge";
            // 订阅期额度充值：数量与金额必填，不依赖目录套餐
            if (isRecharge) {
                const grantQuantity = input.grantQuantity ?? 0;
                const amount = input.amountCny?.trim() ?? "";
                if (grantQuantity <= 0 || !amount || Number.isNaN(Number(amount)) || Number(amount) <= 0) {
                    throw new Error("Membership quota recharge requires a positive grantQuantity and amount.");
                }
            }
            const packageId = String(input.packageId).trim();
            if (!isRecharge && (!packageId || input.packageId <= 0)) {
                throw new Error("A valid membership package is required.");
            }
            const paymentProduct = input.paymentProduct ?? "mobile_cashier_h5";
            const paymentMethod = normalizeMembershipPaymentMethod(input.paymentMethod, paymentProduct);
            const singleFlightKey = [input.action, packageId, paymentMethod, paymentProduct, input.grantQuantity ?? "", input.amountCny ?? ""].join(":");
            const existing = inFlightCheckouts.get(singleFlightKey);
            if (existing) {
                return existing;
            }
            const body = {
                action: input.action,
                packageId,
                paymentMethod,
                paymentProduct,
                ...(isRecharge
                    ? { grantQuantity: String(input.grantQuantity), amount: input.amountCny?.trim() }
                    : {}),
            };
            const checkout = (async () => {
                const params = createSdkworkIdempotencyParams();
                const response = await resolveAppService().memberships.orders.create(body, params);
                return normalizeMembershipCheckoutPayment(unwrapSdkworkOrderResource(response, "Unable to create membership order."), input.packageId, paymentProduct);
            })();
            inFlightCheckouts.set(singleFlightKey, checkout);
            void checkout.then(() => inFlightCheckouts.delete(singleFlightKey), () => inFlightCheckouts.delete(singleFlightKey));
            return checkout;
        },
        async getCheckoutStatus(orderId) {
            requireSdkworkOrderSession();
            const normalizedOrderId = orderId.trim();
            if (!normalizedOrderId) {
                throw new Error("A membership order id is required.");
            }
            const response = await resolveAppService().orders.paymentSuccess.retrieve(normalizedOrderId);
            const record = unwrapSdkworkOrderResource(response, "Unable to retrieve membership order status.");
            return {
                amountCny: null,
                durationDays: null,
                orderId: normalizedOrderId,
                packageId: null,
                status: record?.paid === true ? "completed" : normalizePointsRechargeStatus(record?.status),
            };
        },
    };
}
export function unwrapSdkworkOrderResource(value, fallbackMessage = "Request failed.") {
    const data = unwrapSdkworkOrderResponse(value, fallbackMessage);
    if (data && typeof data === "object" && "item" in data) {
        return data.item;
    }
    return data;
}
export function unwrapSdkworkOrderPage(value, fallbackMessage = "Request failed.") {
    return unwrapSdkworkOrderListPage(value, fallbackMessage).items;
}
export function unwrapSdkworkOrderListPage(value, fallbackMessage = "Request failed.") {
    const data = unwrapSdkworkOrderResponse(value, fallbackMessage);
    if (Array.isArray(data)) {
        return { items: data };
    }
    return {
        items: Array.isArray(data?.items) ? data.items : [],
        pageInfo: data?.pageInfo,
    };
}
export function resolveSdkworkOffsetPagination(pageInfo, fallbackPage, fallbackPageSize) {
    const pageSize = toSdkworkOrderNumber(pageInfo?.pageSize, fallbackPageSize) || fallbackPageSize;
    const page = toSdkworkOrderNumber(pageInfo?.page, fallbackPage) || fallbackPage;
    const total = toSdkworkOrderNumber(pageInfo?.totalItems);
    const totalPages = pageInfo?.totalPages === undefined
        ? (pageSize > 0 ? Math.ceil(total / pageSize) : 0)
        : toSdkworkOrderNumber(pageInfo?.totalPages);
    return {
        page,
        pageSize,
        total,
        hasMore: Boolean(pageInfo?.hasMore ?? page * pageSize < total),
        totalPages,
    };
}
/** Maps UI kebab-case order status filters to API snake_case wire values. */
export function toApiOrderStatusWire(status) {
    const normalized = status.trim();
    if (!normalized || normalized === "all") {
        return normalized;
    }
    return normalized.replace(/-/g, "_").toLowerCase();
}
export function unwrapSdkworkOrderResponse(value, fallbackMessage = "Request failed.") {
    if (!value || typeof value !== "object") {
        return value;
    }
    if (!("data" in value) && !("code" in value)) {
        return value;
    }
    const envelope = value;
    if (!isSuccessCode(envelope.code)) {
        throw new Error(String(envelope.message || envelope.msg || fallbackMessage).trim());
    }
    return (envelope.data ?? null);
}
export function toSdkworkOrderOptionalString(value) {
    const normalized = typeof value === "string" ? value.trim() : String(value ?? "").trim();
    return normalized || undefined;
}
export function toNullableSdkworkOrderNumber(value) {
    if (typeof value === "number" && Number.isFinite(value)) {
        return value;
    }
    if (typeof value === "string" && value.trim()) {
        const parsed = Number(value);
        return Number.isFinite(parsed) ? parsed : null;
    }
    return null;
}
export function toSdkworkOrderNumber(value, fallback = 0) {
    return toNullableSdkworkOrderNumber(value) ?? fallback;
}
export function formatSdkworkOrderCurrencyCny(value, language = "en-US") {
    return formatMoney(value, { currency: "CNY", locale: language, mode: "symbol" }) ?? "--";
}
export function readSdkworkMediaResource(value) {
    if (!value || typeof value !== "object") {
        return undefined;
    }
    const record = value;
    const kind = typeof record.kind === "string" ? record.kind : undefined;
    const source = typeof record.source === "string" ? record.source : undefined;
    if (!kind || !source) {
        return undefined;
    }
    return { ...record, kind, source };
}
function normalizePointsRechargePackage(value) {
    if (!value || typeof value !== "object") {
        return null;
    }
    const record = value;
    const id = toSdkworkOrderOptionalString(record.id ?? record.packageId);
    if (!id) {
        return null;
    }
    return {
        id,
        priceAmount: toSdkworkOrderNumber(record.priceAmount ?? record.price),
        currencyCode: toSdkworkOrderOptionalString(record.currencyCode) ?? "CNY",
        bonusPoints: toSdkworkOrderNumber(record.bonusPoints),
        grantAmount: toSdkworkOrderNumber(record.grantAmount),
        points: toSdkworkOrderNumber(record.points ?? record.grantAmount ?? record.bonusPoints),
    };
}
function normalizePointsRechargePayment(value) {
    const record = value && typeof value === "object" ? value : {};
    const status = normalizePointsRechargeStatus(record.status ?? record.rechargeStatus ?? record.paymentStatus ?? record.orderStatus);
    const cashierUrl = toSdkworkOrderOptionalString(record.cashierUrl);
    const paymentProduct = toSdkworkOrderOptionalString(record.paymentProduct);
    const providerQrCode = toSdkworkOrderOptionalString(record.qrCode ?? record.qrCodePayload ?? record.providerQrCode);
    return {
        amountCny: toNullableSdkworkOrderNumber(record.amountCny ?? record.amount),
        cashierUrl,
        expiresAt: toSdkworkOrderOptionalString(record.expiresAt),
        orderId: toSdkworkOrderOptionalString(record.orderId ?? record.id),
        orderNo: toSdkworkOrderOptionalString(record.orderNo ?? record.outTradeNo),
        points: toSdkworkOrderNumber(record.points ?? record.grantAmount),
        qrCode: paymentProduct === "mobile_cashier_h5" ? cashierUrl : providerQrCode ?? cashierUrl,
        status,
    };
}
export const createSdkworkCouponRedemptionService = createSdkworkCouponRechargeService;
function normalizeCouponRechargeResult(value) {
    const record = value && typeof value === "object" ? value : {};
    const orderId = toSdkworkOrderOptionalString(record.orderId ?? record.id);
    if (!orderId) {
        throw new Error("Coupon redemption did not return an order id.");
    }
    const status = normalizePointsRechargeStatus(record.status ?? record.fulfillmentStatus ?? record.orderStatus);
    const common = {
        orderId,
        orderNo: toSdkworkOrderOptionalString(record.orderNo ?? record.outTradeNo),
        replayed: record.replayed === true,
        status: status === "completed" ? "completed" : "pending",
    };
    const benefit = record.benefit && typeof record.benefit === "object"
        ? record.benefit
        : {};
    if (benefit.kind === "token_bank_credit") {
        const grantAmount = toSdkworkOrderNumber(benefit.grantAmount);
        if (grantAmount <= 0) {
            throw new Error("Coupon redemption did not return a Token Bank grant.");
        }
        return {
            ...common,
            benefitKind: "token_bank_credit",
            grantAmount,
            targetAsset: "token_bank",
        };
    }
    if (benefit.kind === "points_credit") {
        const grantPoints = toSdkworkOrderNumber(benefit.grantPoints);
        if (grantPoints <= 0) {
            throw new Error("Coupon redemption did not return a points grant.");
        }
        return {
            ...common,
            benefitKind: "points_credit",
            grantPoints,
        };
    }
    if (benefit.kind === "cash_credit") {
        const grantAmount = toSdkworkOrderNumber(benefit.grantAmount);
        if (grantAmount <= 0) {
            throw new Error("Coupon redemption did not return a cash grant.");
        }
        return {
            ...common,
            benefitKind: "cash_credit",
            grantAmount,
        };
    }
    if (benefit.kind === "subscription") {
        const productId = toSdkworkOrderOptionalString(benefit.productId);
        const skuId = toSdkworkOrderOptionalString(benefit.skuId);
        const packageId = toSdkworkOrderOptionalString(benefit.packageId);
        const subscriptionId = toSdkworkOrderOptionalString(benefit.subscriptionId);
        const startsAt = toSdkworkOrderOptionalString(benefit.startsAt);
        const expiresAt = toSdkworkOrderOptionalString(benefit.expiresAt);
        const period = toSdkworkOrderOptionalString(benefit.period);
        const durationDays = toSdkworkOrderNumber(benefit.durationDays);
        const dailyQuota = toSdkworkOrderNumber(benefit.dailyQuota);
        const totalQuota = toSdkworkOrderNumber(benefit.totalQuota);
        if (!productId || !skuId || !packageId || !subscriptionId || !startsAt || !expiresAt
            || !period || !["day", "week", "month", "year"].includes(period)
            || durationDays <= 0 || dailyQuota <= 0 || totalQuota < dailyQuota) {
            throw new Error("Coupon redemption returned an invalid subscription benefit.");
        }
        return {
            ...common,
            benefitKind: "subscription",
            dailyQuota,
            durationDays,
            expiresAt,
            packageId,
            period,
            productId,
            skuId,
            startsAt,
            subscriptionId,
            totalQuota,
        };
    }
    throw new Error("Coupon redemption returned an unsupported benefit.");
}
function normalizeMembershipCheckoutPayment(value, fallbackPackageId, paymentProduct) {
    const record = value && typeof value === "object" ? value : {};
    const paymentParams = record.paymentParams && typeof record.paymentParams === "object"
        ? record.paymentParams
        : {};
    const cashierUrl = toSdkworkOrderOptionalString(record.cashierUrl ?? paymentParams.cashierUrl);
    const providerQrCode = toSdkworkOrderOptionalString(paymentParams.qrCodeUrl
        ?? paymentParams.qrCode
        ?? paymentParams.qrCodePayload
        ?? paymentParams.codeUrl
        ?? record.qrCode
        ?? record.qrCodePayload
        ?? record.codeUrl);
    return {
        action: toSdkworkOrderOptionalString(record.action),
        amountCny: toNullableSdkworkOrderNumber(record.amountCny ?? record.amount),
        cashierUrl,
        durationDays: toNullableSdkworkOrderNumber(record.durationDays),
        expiresAt: toSdkworkOrderOptionalString(record.expiresAt),
        orderId: toSdkworkOrderOptionalString(record.orderId ?? record.id),
        packageId: toNullableSdkworkOrderNumber(record.packageId) ?? fallbackPackageId,
        packageName: toSdkworkOrderOptionalString(record.packageName),
        qrCode: paymentProduct === "mobile_cashier_h5" ? cashierUrl : providerQrCode ?? cashierUrl,
        reused: record.reused === true,
        status: normalizePointsRechargeStatus(record.status ?? record.paymentStatus ?? record.orderStatus),
        targetLevelName: toSdkworkOrderOptionalString(record.targetLevelName ?? record.targetPlanName),
    };
}
function normalizeMembershipPaymentMethod(value, paymentProduct) {
    const normalized = value?.trim().toLowerCase().replace(/-/gu, "_");
    if (normalized) {
        return normalized === "wechat" ? "wechat_pay" : normalized;
    }
    return paymentProduct === "alipay_native" ? "alipay" : "wechat_pay";
}
function normalizePhysicalQuantity(value) {
    if (!Number.isSafeInteger(value) || value <= 0) {
        throw new Error("Physical SKU quantity must be a positive integer.");
    }
    return value;
}
function normalizePhysicalShippingAddress(address) {
    return {
        city: requirePhysicalText("shipping city", address.city),
        countryCode: requirePhysicalText("shipping country code", address.countryCode).toUpperCase(),
        detailAddress: requirePhysicalText("shipping detail address", address.detailAddress),
        district: toSdkworkOrderOptionalString(address.district),
        postalCode: toSdkworkOrderOptionalString(address.postalCode),
        province: requirePhysicalText("shipping province", address.province),
        receiverName: requirePhysicalText("shipping receiver name", address.receiverName),
        receiverPhone: requirePhysicalText("shipping receiver phone", address.receiverPhone),
    };
}
function requirePhysicalText(field, value) {
    const normalized = value.trim();
    if (!normalized) {
        throw new Error(`A valid ${field} is required.`);
    }
    return normalized;
}
function normalizePointsRechargeStatus(value) {
    const status = String(value ?? "").trim().toLowerCase();
    if (["completed", "complete", "paid", "success", "succeeded", "fulfilled"].includes(status)) {
        return "completed";
    }
    if (["failed", "cancelled", "canceled", "closed", "expired", "rejected"].includes(status)) {
        return "failed";
    }
    return "pending";
}
function normalizeSessionToken(value) {
    const normalized = typeof value === "string" ? value.trim() : "";
    return normalized || undefined;
}
function isSuccessCode(code) {
    if (code === undefined || code === null || code === "") {
        return true;
    }
    if (typeof code === "number") {
        return code === 0;
    }
    return String(code).trim() === "0";
}
export function bootstrapSdkworkOrderAppService(input) {
    const transport = createOrderAppTransportClient(input);
    const service = createSdkworkOrderAppService({
        appClient: transport,
    });
    configureSdkworkOrderAppServiceProvider(() => service);
    configureSdkworkOrderSessionTokenProvider(() => {
        if (input.tokenManager) {
            return input.tokenManager.getTokens();
        }
        return {
            accessToken: input.accessToken,
            authToken: input.authToken,
        };
    });
    return service;
}
export { createOrderAppTransportClient, resolveOrderAppApiOrigin, } from "./transport.js";
export { bootstrapSdkworkOrderBackendSdk, createOrderBackendTransportClient, getSdkworkOrderBackendSdkClient, resetSdkworkOrderBackendSdkClient, resolveOrderBackendApiOrigin, } from "./backend-transport.js";
export { createSdkworkIdempotencyParams, } from "./idempotency.js";
//# sourceMappingURL=index.js.map