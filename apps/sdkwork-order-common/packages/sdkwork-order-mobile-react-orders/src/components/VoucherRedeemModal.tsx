import { useTranslation } from "react-i18next";
import React, { useState } from "react";
import { X, QrCode, Search } from "lucide-react";
import { showToast } from "@sdkwork/ui-mobile-react";
import { OrderService } from "../services/OrderService";

interface VoucherRedeemModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const VoucherRedeemModal: React.FC<VoucherRedeemModalProps> = ({
  isOpen,
  onClose,
}) => {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [isScanning, setIsScanning] = useState(false);

  if (!isOpen) return null;

  const handleRedeem = async () => {
    if (!code.trim()) {
      showToast(t("orders.voucher_code_required", "请输入或扫描有效的券码"));
      return;
    }

    // Redeem through the canonical Order App SDK coupon redemption port.
    const res = await OrderService.redeemVoucher(code.trim().toUpperCase());
    if (res.success) {
      showToast(t("orders.voucher_redeem_success", "核销成功"));
      onClose();
      setCode("");
    } else {
      showToast(t("orders.voucher_redeem_failed", "核销失败，请重试"));
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-bg-color">
      <header className="flex items-center justify-between px-2 pt-safe h-[56px] border-b border-border-color shrink-0">
        <div className="flex items-center z-10 flex-1">
          <button onClick={onClose} className="p-2 active:opacity-70">
            <X className="w-6 h-6 text-text-main" />
          </button>
        </div>
        <div className="absolute left-1/2 -translate-x-1/2">
          <h2 className="text-[17px] font-semibold text-text-main">{t("orders.voucher_title", "券码核销")}</h2>
        </div>
        <div className="flex-1" />
      </header>

      <div className="flex-1 overflow-y-auto p-4 flex flex-col items-center">
        {!isScanning ? (
          <>
            <div className="w-full max-w-[320px] bg-chat-other-bg p-4 rounded-xl shadow-sm mb-6 mt-4 border border-border-color/30 flex flex-col gap-4">
              <p className="text-[14px] text-text-main font-medium">{t("orders.voucher_input_label", "输入券码号核销")}</p>
              <div className="flex items-center bg-bg-color rounded-lg px-3 py-2 border border-border-color focus-within:border-primary-blue/50 transition-colors">
                <Search className="w-4 h-4 text-text-sub shrink-0" />
                <input
                  type="text"
                  placeholder={t("orders.voucher_input_placeholder", "请输入如: ABC12345")}
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  className="flex-1 bg-transparent border-none outline-none text-[15px] text-text-main px-2 min-w-0"
                />
              </div>
              <button
                onClick={handleRedeem}
                className="w-full bg-[#FA5151] active:opacity-80 text-white font-medium text-[15px] py-2.5 rounded-lg transition-opacity"
              >{t("orders.voucher_confirm", "确认核销")}</button>
            </div>

            <div className="flex items-center gap-4 w-full max-w-[320px] my-4">
              <div className="flex-1 h-[1px] bg-border-color"></div>
              <span className="text-[13px] text-text-sub font-medium">{t("orders.voucher_or", "或")}</span>
              <div className="flex-1 h-[1px] bg-border-color"></div>
            </div>

            <div 
              onClick={() => setIsScanning(true)}
              className="w-full max-w-[320px] bg-chat-other-bg active:bg-active-bg p-6 rounded-xl flex flex-col items-center justify-center gap-3 border border-border-color/30 transition-colors cursor-pointer"
            >
              <div className="w-14 h-14 bg-primary-blue/10 text-primary-blue rounded-full flex items-center justify-center mb-1">
                <QrCode className="w-7 h-7" />
              </div>
              <span className="text-[15px] font-medium text-text-main">{t("orders.voucher_scan", "扫码核销")}</span>
              <span className="text-[13px] text-text-sub text-center">{t("orders.voucher_scan_desc", "扫描客户出示的二维码或条形码")}</span>
            </div>
          </>
        ) : (
          <div className="flex-1 w-full bg-black flex flex-col relative items-center justify-center">
            {/* Mock Scanner View */}
            <div className="absolute inset-x-8 top-1/4 bottom-1/3 border-2 border-primary-blue/50 rounded-xl flex items-center justify-center overflow-hidden bg-white/5 backdrop-blur-sm shadow-[0_0_0_1000px_rgba(0,0,0,0.5)]">
              <div className="w-full h-[2px] bg-primary-blue/80 blur-[1px] absolute top-0 animate-[scan_2s_ease-in-out_infinite]" style={{ boxShadow: '0 0 10px 2px rgba(x,y,z,0.5)' }}></div>
              <span className="text-white/70 text-[13px] absolute bottom-4 drop-shadow-md">{t("orders.voucher_scan_align", "将二维码/条码对准框内")}</span>
            </div>
            
            <button
              onClick={() => setIsScanning(false)}
              className="absolute bottom-12 bg-white/10 active:bg-white/20 px-6 py-2 rounded-full text-white text-[15px] backdrop-blur-md transition-colors"
            >{t("orders.voucher_cancel_scan", "取消扫码")}</button>
          </div>
        )}
      </div>

      <style>{`
        @keyframes scan {
          0%, 100% { top: 0; }
          50% { top: 100%; }
        }
      `}</style>
    </div>
  );
};
