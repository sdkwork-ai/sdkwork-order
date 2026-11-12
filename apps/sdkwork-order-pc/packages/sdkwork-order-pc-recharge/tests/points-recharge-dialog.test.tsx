import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SdkworkThemeProvider } from "@sdkwork/ui-pc-react/theme";
import {
  SdkworkPointsRechargeDialog,
  SdkworkPointsRechargeInline,
  type SdkworkPointsRechargeService,
} from "../src";

vi.mock("qrcode", () => ({
  toDataURL: vi.fn().mockResolvedValue("data:image/png;base64,recharge-qr"),
}));

describe("SDKWork points recharge surfaces", () => {
  it("selects a recharge package id and completes order creation inside the dialog", async () => {
    const createOrder = vi.fn().mockResolvedValue({
      amountCny: 75,
      orderId: "recharge-order-750",
      points: 750_000_000,
      qrCode: "weixin://pay/recharge-order-750",
      status: "pending" as const,
    });
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
        { id: "recharge-750", bonusPoints: 0, currencyCode: "CNY", grantAmount: 750, points: 750_000_000, priceAmount: 75 },
      ]),
      createOrder,
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn().mockResolvedValue({
        amountCny: 75,
        orderId: "recharge-order-750",
        points: 750_000_000,
        status: "pending" as const,
      }),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog
          copy={{ account: "Cloud Router", title: "Buy Compute Credits" }}
          isOpen
          onClose={vi.fn()}
          service={service}
        />
      </SdkworkThemeProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /750/ }));
    fireEvent.click(screen.getByRole("button", { name: "同意并支付" }));

    await waitFor(() => expect(createOrder).toHaveBeenCalledWith(expect.objectContaining({
      packageId: "recharge-750",
      paymentMethod: "wechat_pay",
    })));
    expect(await screen.findByRole("img", { name: "请扫码完成支付" })).toHaveAttribute(
      "src",
      "data:image/png;base64,recharge-qr",
    );
    expect(screen.getByRole("dialog", { name: "Buy Compute Credits" })).toBeInTheDocument();
    expect(screen.getAllByText("¥75.00")).toHaveLength(1);
    expect(document.querySelector(".sdkwork-points-recharge-dialog__payment")).not.toHaveTextContent("¥75.00");
    const qrAgreement = screen.getByText("您已同意《积分充值服务协议》");
    expect(qrAgreement).toBeInTheDocument();
    expect(qrAgreement.querySelector(".sdkwork-points-recharge-dialog__agreement-check svg")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "同意并支付" })).not.toBeInTheDocument();
    view.unmount();
  });

  it("shows the package price in major currency units instead of dividing it again", async () => {
    // `GET /app/v3/api/recharges/packages` already serializes the stored minor
    // units as a major amount, so a 500-point package priced at 50.00 CNY (the
    // platform 1 CNY = 10 points ratio) must render ¥50.00, never ¥0.50.
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-100", bonusPoints: 0, currencyCode: "CNY", grantAmount: 100, points: 100_000_000, priceAmount: 10 },
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
        { id: "recharge-50000", bonusPoints: 0, currencyCode: "CNY", grantAmount: 50000, points: 50_000_000_000, priceAmount: 5000 },
      ]),
      createOrder: vi.fn(),
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn(),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog isOpen onClose={vi.fn()} service={service} />
      </SdkworkThemeProvider>,
    );

    expect(await screen.findByText("¥10.00")).toBeInTheDocument();
    expect(screen.getByText("¥50.00")).toBeInTheDocument();
    expect(screen.getByText("¥5,000.00")).toBeInTheDocument();
    expect(screen.queryByText("¥0.10")).not.toBeInTheDocument();
    expect(screen.queryByText("¥0.50")).not.toBeInTheDocument();
    view.unmount();
  });

  it("buys a custom amount that the server prices from the exchange rule", async () => {
    const createOrder = vi.fn().mockResolvedValue({
      amountCny: 88.5,
      orderId: "recharge-order-custom",
      points: 885_000_000,
      qrCode: "weixin://pay/recharge-order-custom",
      status: "pending" as const,
    });
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
      ]),
      createOrder,
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn(),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog isOpen onClose={vi.fn()} service={service} />
      </SdkworkThemeProvider>,
    );

    await screen.findByText("我的积分");
    fireEvent.click(await screen.findByRole("button", { name: /自定义金额/ }));
    const amountField = screen.getByLabelText("自定义金额");
    expect(screen.getByRole("button", { name: "同意并支付" })).toBeDisabled();

    fireEvent.change(amountField, { target: { value: "88.5" } });
    expect(await screen.findByText("预计到账 885 积分")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "同意并支付" }));

    await waitFor(() => expect(createOrder).toHaveBeenCalledWith(expect.objectContaining({
      amount: 88.5,
      paymentMethod: "wechat_pay",
    })));
    // A free amount never claims a package the server cannot verify.
    expect(createOrder.mock.calls[0]?.[0]).not.toHaveProperty("packageId");
    view.unmount();
  });

  it("blocks an out-of-range custom amount before creating an order", async () => {
    const createOrder = vi.fn();
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
      ]),
      createOrder,
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn(),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog isOpen onClose={vi.fn()} service={service} />
      </SdkworkThemeProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /自定义金额/ }));
    fireEvent.change(screen.getByLabelText("自定义金额"), { target: { value: "10000.01" } });

    expect(await screen.findByRole("alert")).toHaveTextContent("请输入 1.00 - 10000.00 元之间的金额");
    const payButton = screen.getByRole("button", { name: "同意并支付" });
    expect(payButton).toBeDisabled();
    fireEvent.click(payButton);
    expect(createOrder).not.toHaveBeenCalled();
    view.unmount();
  });

  it("keeps agreement for the current dialog session and pays a newly selected package automatically", async () => {
    let resolveFirstOrder!: (payment: {
      amountCny: number;
      orderId: string;
      points: number;
      qrCode: string;
      status: "pending";
    }) => void;
    const firstOrder = new Promise<{
      amountCny: number;
      orderId: string;
      points: number;
      qrCode: string;
      status: "pending";
    }>((resolve) => {
      resolveFirstOrder = resolve;
    });
    const createOrder = vi.fn()
      .mockReturnValueOnce(firstOrder)
      .mockResolvedValueOnce({
        amountCny: 50,
        orderId: "recharge-order-500",
        points: 500_000_000,
        qrCode: "weixin://pay/recharge-order-500",
        status: "pending" as const,
      });
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
        { id: "recharge-750", bonusPoints: 0, currencyCode: "CNY", grantAmount: 750, points: 750_000_000, priceAmount: 75 },
      ]),
      createOrder,
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn().mockImplementation(async (orderId: string) => ({
        amountCny: orderId.endsWith("500") ? 50 : 75,
        orderId,
        points: orderId.endsWith("500") ? 500 : 750,
        status: "pending" as const,
      })),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog isOpen onClose={vi.fn()} service={service} />
      </SdkworkThemeProvider>,
    );

    const package750 = await screen.findByRole("button", { name: /750/ });
    const package500 = screen.getByRole("button", { name: /500/ });
    fireEvent.click(package750);
    const confirmPayment = screen.getByRole("button", { name: "同意并支付" });
    fireEvent.click(confirmPayment);
    fireEvent.click(confirmPayment);

    await waitFor(() => {
      expect(createOrder).toHaveBeenCalledTimes(1);
      expect(package750).toBeDisabled();
      expect(package500).toBeDisabled();
    });
    expect(screen.getByText("您已同意《积分充值服务协议》")
      .querySelector(".sdkwork-points-recharge-dialog__agreement-check svg")).toBeInTheDocument();
    fireEvent.click(package500);
    expect(package750).toHaveAttribute("aria-pressed", "true");

    await act(async () => {
      resolveFirstOrder({
        amountCny: 75,
        orderId: "recharge-order-750",
        points: 750_000_000,
        qrCode: "weixin://pay/recharge-order-750",
        status: "pending",
      });
      await firstOrder;
    });

    expect(await screen.findByRole("img", { name: "请扫码完成支付" })).toBeInTheDocument();
    fireEvent.click(package500);

    expect(screen.queryByRole("img", { name: "请扫码完成支付" })).not.toBeInTheDocument();
    const acceptedAgreement = screen.getByText("您已同意《积分充值服务协议》");
    expect(acceptedAgreement).toBeInTheDocument();
    expect(acceptedAgreement.querySelector(".sdkwork-points-recharge-dialog__agreement-check svg")).toBeInTheDocument();
    await waitFor(() => expect(createOrder).toHaveBeenNthCalledWith(2, expect.objectContaining({
      packageId: "recharge-500",
      paymentMethod: "wechat_pay",
    })));
    view.unmount();
  });

  it("reuses a live recharge order and replaces it when the selected package is clicked after expiry", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-27T04:00:00.000Z"));
    const createOrder = vi
      .fn()
      .mockResolvedValueOnce({
        amountCny: 50,
        expiresAt: "2026-07-27T04:00:02.000Z",
        orderId: "recharge-order-expiring",
        points: 500_000_000,
        qrCode: "data:image/png;base64,recharge-expiring",
        status: "pending" as const,
      })
      .mockResolvedValueOnce({
        amountCny: 50,
        expiresAt: "2026-07-27T04:05:00.000Z",
        orderId: "recharge-order-replacement",
        points: 500_000_000,
        qrCode: "data:image/png;base64,recharge-replacement",
        status: "pending" as const,
      });
    const getOrderStatus = vi.fn().mockImplementation(async (orderId: string) => ({
      amountCny: 50,
      expiresAt: orderId === "recharge-order-expiring"
        ? "2026-07-27T04:00:02.000Z"
        : "2026-07-27T04:05:00.000Z",
      orderId,
      points: 500_000_000,
      status: "pending" as const,
    }));
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
      ]),
      createOrder,
      getOrderStatus,
    };

    let view: ReturnType<typeof render> | undefined;
    try {
      view = render(
        <SdkworkThemeProvider defaultTheme="light">
          <SdkworkPointsRechargeDialog
            copy={{
              confirmPayment: "Accept and pay",
              expired: "Order expired",
              expiredDescription: "Create a new order to continue.",
              expiresIn: "Order expires in",
              retryPayment: "Create new order",
              scanPrompt: "Scan to complete payment",
            }}
            isOpen
            onClose={vi.fn()}
            service={service}
          />
        </SdkworkThemeProvider>,
      );

      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      fireEvent.click(screen.getByRole("button", { name: "Accept and pay" }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });

      expect(screen.getByRole("timer")).toHaveTextContent("Order expires in00:02");
      expect(screen.getByRole("img", { name: "Scan to complete payment" })).toBeInTheDocument();
      expect(screen.getAllByText("¥50.00")).toHaveLength(1);
      fireEvent.click(screen.getByRole("button", { name: /500/ }));
      expect(createOrder).toHaveBeenCalledTimes(1);

      await act(async () => {
        vi.advanceTimersByTime(2_000);
        await Promise.resolve();
      });
      expect(screen.getByText("Order expired")).toBeInTheDocument();
      expect(screen.queryByRole("img", { name: "Scan to complete payment" })).not.toBeInTheDocument();
      const statusReadsAtExpiration = getOrderStatus.mock.calls.length;

      await act(async () => {
        vi.advanceTimersByTime(5_000);
        await Promise.resolve();
      });
      expect(getOrderStatus).toHaveBeenCalledTimes(statusReadsAtExpiration);

      fireEvent.click(screen.getByRole("button", { name: /500/ }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(createOrder).toHaveBeenCalledTimes(2);
      expect(screen.getByRole("img", { name: "Scan to complete payment" })).toHaveAttribute(
        "src",
        "data:image/png;base64,recharge-replacement",
      );
    } finally {
      view?.unmount();
      act(() => vi.runOnlyPendingTimers());
      vi.useRealTimers();
    }
  });

  it("requires agreement again after the dialog is closed and reopened", async () => {
    const createOrder = vi.fn().mockResolvedValue({
      amountCny: 50,
      orderId: "recharge-order-500",
      points: 500_000_000,
      qrCode: "weixin://pay/recharge-order-500",
      status: "pending" as const,
    });
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
      ]),
      createOrder,
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn().mockResolvedValue({
        amountCny: 50,
        orderId: "recharge-order-500",
        points: 500_000_000,
        status: "pending" as const,
      }),
    };
    const renderDialog = (isOpen: boolean) => (
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog isOpen={isOpen} onClose={vi.fn()} service={service} />
      </SdkworkThemeProvider>
    );
    const view = render(renderDialog(true));

    fireEvent.click(await screen.findByRole("button", { name: "同意并支付" }));
    await waitFor(() => expect(createOrder).toHaveBeenCalledTimes(1));

    view.rerender(renderDialog(false));
    view.rerender(renderDialog(true));

    expect(await screen.findByRole("button", { name: "同意并支付" })).toBeInTheDocument();
    expect(createOrder).toHaveBeenCalledTimes(1);
    view.unmount();
  });

  it("ignores a stale completion poll after the customer switches packages", async () => {
    let resolveStatus!: (payment: {
      amountCny: number;
      orderId: string;
      points: number;
      status: "completed";
    }) => void;
    const statusResult = new Promise<{
      amountCny: number;
      orderId: string;
      points: number;
      status: "completed";
    }>((resolve) => {
      resolveStatus = resolve;
    });
    const onCompleted = vi.fn();
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
        { id: "recharge-750", bonusPoints: 0, currencyCode: "CNY", grantAmount: 750, points: 750_000_000, priceAmount: 75 },
      ]),
      createOrder: vi.fn()
        .mockResolvedValueOnce({
          amountCny: 75,
          orderId: "recharge-order-750",
          points: 750_000_000,
          qrCode: "weixin://pay/recharge-order-750",
          status: "pending" as const,
        })
        .mockResolvedValueOnce({
          amountCny: 50,
          orderId: "recharge-order-500",
          points: 500_000_000,
          qrCode: "weixin://pay/recharge-order-500",
          status: "pending" as const,
        }),
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn().mockImplementation((orderId: string) => orderId === "recharge-order-750"
        ? statusResult
        : Promise.resolve({
            amountCny: 50,
            orderId,
            points: 500_000_000,
            status: "pending" as const,
          })),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeDialog
          isOpen
          onClose={vi.fn()}
          onCompleted={onCompleted}
          service={service}
        />
      </SdkworkThemeProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /750/ }));
    fireEvent.click(screen.getByRole("button", { name: "同意并支付" }));
    expect(await screen.findByRole("img", { name: "请扫码完成支付" })).toBeInTheDocument();
    await waitFor(() => expect(service.getOrderStatus).toHaveBeenCalledWith("recharge-order-750"));

    fireEvent.click(screen.getByRole("button", { name: /500/ }));
    await waitFor(() => expect(service.createOrder).toHaveBeenLastCalledWith(expect.objectContaining({
      packageId: "recharge-500",
      paymentMethod: "wechat_pay",
    })));
    await act(async () => {
      resolveStatus({
        amountCny: 75,
        orderId: "recharge-order-750",
        points: 750_000_000,
        status: "completed",
      });
      await statusResult;
    });

    expect(screen.getByRole("button", { name: /500/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("您已同意《积分充值服务协议》")).toBeInTheDocument();
    expect(screen.queryByText("支付完成，积分已到账")).not.toBeInTheDocument();
    expect(onCompleted).not.toHaveBeenCalled();
    view.unmount();
  });

  it("provides the dialog checkout flow as an inline surface", async () => {
    const service: SdkworkPointsRechargeService = {
      listPackages: vi.fn().mockResolvedValue([
        { id: "recharge-500", bonusPoints: 0, currencyCode: "CNY", grantAmount: 500, points: 500_000_000, priceAmount: 50 },
      ]),
      createOrder: vi.fn().mockResolvedValue({
        amountCny: 50,
        orderId: "recharge-order-inline",
        points: 500_000_000,
        qrCode: "weixin://pay/recharge-order-inline",
        status: "pending" as const,
      }),
      readSettings: vi.fn().mockResolvedValue({
        baseCurrencyCode: "CNY",
        basePointsPerCny: 10,
        currencyToCnyRates: { CNY: "1" },
      }),
      getOrderStatus: vi.fn().mockResolvedValue({
        amountCny: 50,
        orderId: "recharge-order-inline",
        points: 500_000_000,
        status: "pending" as const,
      }),
    };

    const view = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkPointsRechargeInline currentPoints={1250} service={service} />
      </SdkworkThemeProvider>,
    );

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "积分账户" })).toHaveClass("sdkwork-points-recharge-inline");
    expect(await screen.findByRole("button", { name: /500/ })).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(screen.getByRole("button", { name: "同意并支付" }));

    expect(await screen.findByRole("img", { name: "请扫码完成支付" })).toBeInTheDocument();
    expect(service.createOrder).toHaveBeenCalledWith(expect.objectContaining({
      packageId: "recharge-500",
      paymentMethod: "wechat_pay",
    }));
    view.unmount();
  });
});
