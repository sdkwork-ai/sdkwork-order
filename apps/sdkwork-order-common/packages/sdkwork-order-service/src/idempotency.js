import { uuid } from "@sdkwork/utils";
export function createSdkworkIdempotencyParams(idempotencyKey) {
    return { idempotencyKey: idempotencyKey ?? uuid() };
}
//# sourceMappingURL=idempotency.js.map