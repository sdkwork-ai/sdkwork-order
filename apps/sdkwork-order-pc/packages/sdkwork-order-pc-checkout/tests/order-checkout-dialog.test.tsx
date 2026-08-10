import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SdkworkThemeProvider } from "@sdkwork/ui-pc-react/theme";
import {
  SdkworkOrderCheckoutDialog,
  type SdkworkOrderCheckoutDialogCopy,
  type SdkworkOrderCheckoutPayment,
} from "../src";

vi.mock("qrcode", () => ({
  toDataURL: vi.fn().mockResolvedValue("data:image/png;base64,checkout-qr"),
}));

const copy = {
  activationDescription: "Entitlement is synchronized after payment.",
  activationTitle: "Instant activation",
  close: "Close",
  completed: "Payment completed",
  creatingPayment: "Creating payment QR code...",
  expired: "Order expired",
  expiredDescription: "This order has expired. Create a new order to continue.",
  expiresIn: "Order expires in",
  paymentUnavailable: "Payment QR code unavailable",
  paymentUnavailableDescription: "The payment QR code is unavailable.",
  payByQr: "Scan to pay",
  price: "Price",
  retry: "Retry",
  scanPrompt: "Scan with a mobile payment app",
  secureDescription: "Payment data is used for this order only.",
  secureTitle: "Secure checkout",
  selectedItem: "Selected item",
  title: "Purchase plan",
} satisfies SdkworkOrderCheckoutDialogCopy;

describe("SdkworkOrderCheckoutDialog", () => {
  it("creates one payment and renders its QR payload in the order-owned payment panel", async () => {
    const createPayment = vi.fn().mockResolvedValue({
      orderId: "ORDER-1",
      qrCode: "weixin://payment/ORDER-1",
      status: "pending" as const,
    });

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{ createPayment }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            periodLabel: "Monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    expect(await screen.findByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
      "src",
      "data:image/png;base64,checkout-qr",
    );
    await waitFor(() => expect(createPayment).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Super monthly")).toBeInTheDocument();
    expect(screen.getByText(copy.secureTitle)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: copy.title })).toHaveTextContent(copy.title);
    expect(screen.getByRole("heading", { name: copy.title })).not.toHaveTextContent("\u00a5199");
    expect(screen.getAllByText("\u00a5199")).toHaveLength(1);
    expect(document.querySelector('[data-sdk-region="order-checkout-payment"]')).not.toHaveTextContent("\u00a5199");
  });

  it("counts down from the server-provided order expiration time", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-27T04:00:00.000Z"));

    try {
      render(
        <SdkworkThemeProvider defaultTheme="light">
          <SdkworkOrderCheckoutDialog
            copy={copy}
            driver={{
              createPayment: async () => ({
                expiresAt: "2026-07-27T04:01:05.000Z",
                orderId: "ORDER-COUNTDOWN",
                qrCode: "data:image/png;base64,countdown-qr",
                status: "pending",
              }),
            }}
            isOpen
            onClose={vi.fn()}
            summary={{
              id: "membership-super-monthly",
              name: "Super monthly",
              priceLabel: "\u00a5199",
            }}
          />
        </SdkworkThemeProvider>,
      );

      await act(async () => {
        await Promise.resolve();
      });
      expect(screen.getByRole("timer")).toHaveTextContent(`${copy.expiresIn}01:05`);

      act(() => vi.advanceTimersByTime(5_000));
      expect(screen.getByRole("timer")).toHaveTextContent(`${copy.expiresIn}01:00`);
    } finally {
      vi.useRealTimers();
    }
  });

  it("uses a revised expiration time returned by payment polling", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-27T04:00:00.000Z"));
    const getPaymentStatus = vi.fn().mockResolvedValue({
      expiresAt: "2026-07-27T04:02:00.000Z",
      orderId: "ORDER-REVISED-EXPIRATION",
      qrCode: "data:image/png;base64,revised-expiration-qr",
      status: "pending" as const,
    });

    try {
      render(
        <SdkworkThemeProvider defaultTheme="light">
          <SdkworkOrderCheckoutDialog
            copy={copy}
            driver={{
              createPayment: async () => ({
                expiresAt: "2026-07-27T04:01:00.000Z",
                orderId: "ORDER-REVISED-EXPIRATION",
                qrCode: "data:image/png;base64,revised-expiration-qr",
                status: "pending",
              }),
              getPaymentStatus,
              pollIntervalMs: 60_000,
            }}
            isOpen
            onClose={vi.fn()}
            summary={{
              id: "membership-super-monthly",
              name: "Super monthly",
              priceLabel: "\u00a5199",
            }}
          />
        </SdkworkThemeProvider>,
      );

      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(getPaymentStatus).toHaveBeenCalled();
      expect(screen.getByRole("timer")).toHaveTextContent(`${copy.expiresIn}02:00`);
    } finally {
      vi.useRealTimers();
    }
  });

  it("invalidates an expired QR code and creates a new order when retried", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-27T04:00:00.000Z"));
    const createPayment = vi
      .fn()
      .mockResolvedValueOnce({
        expiresAt: "2026-07-27T04:00:02.000Z",
        orderId: "ORDER-EXPIRING",
        qrCode: "data:image/png;base64,expiring-qr",
        status: "pending" as const,
      })
      .mockResolvedValueOnce({
        expiresAt: "2026-07-27T04:05:00.000Z",
        orderId: "ORDER-REPLACEMENT",
        qrCode: "data:image/png;base64,replacement-qr",
        status: "pending" as const,
      });
    const getPaymentStatus = vi.fn().mockImplementation(
      async (payment: SdkworkOrderCheckoutPayment) => payment,
    );

    try {
      render(
        <SdkworkThemeProvider defaultTheme="light">
          <SdkworkOrderCheckoutDialog
            copy={copy}
            driver={{ createPayment, getPaymentStatus, pollIntervalMs: 1_000 }}
            isOpen
            onClose={vi.fn()}
            summary={{
              id: "membership-super-monthly",
              name: "Super monthly",
              priceLabel: "\u00a5199",
            }}
          />
        </SdkworkThemeProvider>,
      );

      await act(async () => {
        await Promise.resolve();
      });
      expect(screen.getByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
        "src",
        "data:image/png;base64,expiring-qr",
      );

      await act(async () => {
        vi.advanceTimersByTime(2_000);
        await Promise.resolve();
      });
      expect(screen.queryByRole("img", { name: copy.scanPrompt })).not.toBeInTheDocument();
      expect(screen.getByText(copy.expired)).toBeInTheDocument();
      const statusReadsAtExpiration = getPaymentStatus.mock.calls.length;

      await act(async () => {
        vi.advanceTimersByTime(5_000);
        await Promise.resolve();
      });
      expect(getPaymentStatus).toHaveBeenCalledTimes(statusReadsAtExpiration);

      fireEvent.click(screen.getByRole("button", { name: copy.retry }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(createPayment).toHaveBeenCalledTimes(2);
      expect(screen.getByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
        "src",
        "data:image/png;base64,replacement-qr",
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it("uses the mobile cashier URL as the QR payload when qrCode is omitted", async () => {
    const cashierUrl = "https://im.sdkwork.com/cashier/ORDER-H5?scene=virtual&outTradeNo=TRADE-H5";

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{
            createPayment: async () => ({
              cashierUrl,
              orderId: "ORDER-H5",
              status: "pending",
            }),
          }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-standard-monthly",
            name: "Standard monthly",
            priceLabel: "\u00a5158",
          }}
        />
      </SdkworkThemeProvider>,
    );

    expect(await screen.findByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
      "src",
      "data:image/png;base64,checkout-qr",
    );
    expect(screen.queryByText(copy.paymentUnavailable)).not.toBeInTheDocument();
  });

  it("keeps the selected plan on the left and the QR payment panel on the right for PC checkout", async () => {
    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{
            createPayment: async () => ({
              orderId: "ORDER-PC-LAYOUT",
              qrCode: "data:image/png;base64,checkout-qr",
              status: "pending",
            }),
          }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    await screen.findByRole("img", { name: copy.scanPrompt });

    const summaryPanel = document.querySelector('[data-sdk-region="order-checkout-summary"]');
    const paymentPanel = document.querySelector('[data-sdk-region="order-checkout-payment"]');

    expect(summaryPanel).toBeInTheDocument();
    expect(paymentPanel).toBeInTheDocument();
    expect(paymentPanel).toHaveClass("sdkwork-order-checkout-dialog__payment-panel");
    expect(summaryPanel?.parentElement).toHaveClass(
      "sdkwork-order-checkout-dialog__body",
    );
  });

  it("polls the injected payment-status driver and reports a completed payment once", async () => {
    const createPayment = vi.fn().mockResolvedValue({
      orderId: "ORDER-2",
      qrCode: "data:image/png;base64,pending-qr",
      status: "pending" as const,
    });
    const getPaymentStatus = vi.fn().mockResolvedValue({
      orderId: "ORDER-2",
      status: "completed" as const,
    });
    const onPaymentCompleted = vi.fn();

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{ createPayment, getPaymentStatus, onPaymentCompleted }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    expect(await screen.findByText(copy.completed)).toBeInTheDocument();
    await waitFor(() => expect(getPaymentStatus).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onPaymentCompleted).toHaveBeenCalledTimes(1));
    expect(onPaymentCompleted).toHaveBeenCalledWith(
      expect.objectContaining({ orderId: "ORDER-2", status: "completed" }),
    );
  });

  it("does not recreate a payment when localized copy changes", async () => {
    const createPayment = vi.fn().mockResolvedValue({
      orderId: "ORDER-3",
      qrCode: "data:image/png;base64,pending-qr",
      status: "pending" as const,
    });
    const driver = { createPayment };
    const summary = {
      id: "membership-super-monthly",
      name: "Super monthly",
      priceLabel: "\u00a5199",
    };
    const { rerender } = render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={driver}
          isOpen
          onClose={vi.fn()}
          summary={summary}
        />
      </SdkworkThemeProvider>,
    );

    await waitFor(() => expect(createPayment).toHaveBeenCalledTimes(1));
    rerender(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={{ ...copy, paymentUnavailableDescription: "Localized payment error." }}
          driver={driver}
          isOpen
          onClose={vi.fn()}
          summary={summary}
        />
      </SdkworkThemeProvider>,
    );

    await waitFor(() => expect(createPayment).toHaveBeenCalledTimes(1));
  });

  it("clears an old QR image when the payment status explicitly invalidates it", async () => {
    let resolveStatus: ((value: {
      orderId: string;
      qrCode: undefined;
      status: "pending";
    }) => void) | undefined;
    const getPaymentStatus = vi.fn().mockImplementation(
      () => new Promise((resolve) => {
        resolveStatus = resolve;
      }),
    );

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{
            createPayment: async () => ({
              orderId: "ORDER-4",
              qrCode: "data:image/png;base64,initial-qr",
              status: "pending",
            }),
            getPaymentStatus,
          }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    expect(await screen.findByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
      "src",
      "data:image/png;base64,initial-qr",
    );
    await waitFor(() => expect(getPaymentStatus).toHaveBeenCalledTimes(1));

    await act(async () => {
      resolveStatus?.({ orderId: "ORDER-4", qrCode: undefined, status: "pending" });
    });

    await waitFor(() => {
      expect(screen.queryByRole("img", { name: copy.scanPrompt })).not.toBeInTheDocument();
    });
  });

  it("refreshes an existing order before retrying checkout creation", async () => {
    const createPayment = vi.fn().mockResolvedValue({
      orderId: "ORDER-RETRY",
      status: "pending" as const,
    });
    const getPaymentStatus = vi
      .fn()
      .mockResolvedValueOnce({
        orderId: "ORDER-RETRY",
        status: "pending" as const,
      })
      .mockResolvedValueOnce({
        orderId: "ORDER-RETRY",
        qrCode: "data:image/png;base64,refreshed-qr",
        status: "pending" as const,
      });

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{ createPayment, getPaymentStatus }}
          isOpen
          onClose={vi.fn()}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    const retry = await screen.findByRole("button", { name: copy.retry });
    await waitFor(() => expect(getPaymentStatus).toHaveBeenCalledTimes(1));
    fireEvent.click(retry);

    expect(await screen.findByRole("img", { name: copy.scanPrompt })).toHaveAttribute(
      "src",
      "data:image/png;base64,refreshed-qr",
    );
    expect(getPaymentStatus).toHaveBeenCalledTimes(2);
    expect(createPayment).toHaveBeenCalledTimes(1);
  });

  it("delegates Escape dismissal to the design-system modal", async () => {
    const onClose = vi.fn();

    render(
      <SdkworkThemeProvider defaultTheme="light">
        <SdkworkOrderCheckoutDialog
          copy={copy}
          driver={{
            createPayment: async () => ({
              orderId: "ORDER-5",
              qrCode: "data:image/png;base64,pending-qr",
              status: "pending",
            }),
          }}
          isOpen
          onClose={onClose}
          summary={{
            id: "membership-super-monthly",
            name: "Super monthly",
            priceLabel: "\u00a5199",
          }}
        />
      </SdkworkThemeProvider>,
    );

    await screen.findByRole("dialog");
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
