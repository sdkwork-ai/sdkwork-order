import { describe, expect, it, vi } from "vitest";

import { createWithdrawalRequestService } from "../src/services/WithdrawalRequestService";
import type { SdkworkOrderAppService } from "@sdkwork/order-service";

function appServiceStub(overrides: Partial<SdkworkOrderAppService> = {}) {
  return {
    withdrawals: {
      requests: {
        create: vi.fn().mockResolvedValue({
          code: 0,
          data: {
            item: {
              id: "withdrawal-1",
              requestNo: "WD-0001",
              status: "requested",
            },
          },
          traceId: "trace-1",
        }),
        retrieve: vi.fn().mockResolvedValue({
          code: 0,
          data: {
            item: {
              id: "withdrawal-1",
              requestNo: "WD-0001",
              status: "paid_out",
            },
          },
          traceId: "trace-2",
        }),
      },
    },
    ...overrides,
  } as unknown as SdkworkOrderAppService;
}

describe("withdrawal request service", () => {
  it("creates a cash withdrawal through the order app SDK with idempotency params", async () => {
    const appService = appServiceStub();
    const service = createWithdrawalRequestService({ appService });

    const result = await service.createWithdrawalRequest({
      amount: "100.00",
      currencyCode: "CNY",
      payoutMethod: "bank_card",
      payoutAccountRef: "bank-account-1",
    });

    expect(appService.withdrawals.requests.create).toHaveBeenCalledTimes(1);
    const [body, params] = vi.mocked(appService.withdrawals.requests.create).mock.calls[0];
    expect(body).toMatchObject({
      asset: "cash",
      amount: "100.00",
      currencyCode: "CNY",
      payoutMethod: "bank_card",
      payoutAccountRef: "bank-account-1",
    });
    expect(params).toHaveProperty("idempotencyKey");
    expect(result.withdrawalRequestId).toBe("withdrawal-1");
    expect(result.requestNo).toBe("WD-0001");
    expect(result.status).toBe("requested");
  });

  it("unwraps the SdkWork envelope data.item when normalizing the record", async () => {
    const appService = appServiceStub();
    const service = createWithdrawalRequestService({ appService });

    const result = await service.retrieveWithdrawalRequest("withdrawal-1");

    expect(appService.withdrawals.requests.retrieve).toHaveBeenCalledWith("withdrawal-1");
    expect(result.withdrawalRequestId).toBe("withdrawal-1");
    expect(result.status).toBe("paid_out");
  });

  it("tolerates already-unwrapped payloads", async () => {
    const appService = appServiceStub({
      withdrawals: {
        requests: {
          create: vi.fn().mockResolvedValue({ id: "withdrawal-2", status: "rejected" }),
          retrieve: vi.fn(),
        },
      },
    } as unknown as SdkworkOrderAppService);
    const service = createWithdrawalRequestService({ appService });

    const result = await service.createWithdrawalRequest({
      amount: "1.00",
      currencyCode: "CNY",
    });

    expect(result.withdrawalRequestId).toBe("withdrawal-2");
    expect(result.status).toBe("rejected");
  });
});
