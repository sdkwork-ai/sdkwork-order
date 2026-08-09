import React from "react";
import { cn } from "@sdkwork/ui-mobile-react";
import type { MembershipPackageGroup } from "../services/SubscriptionCatalogPort";

interface VipPlanTabsProps {
  groups: MembershipPackageGroup[];
  activeGroupId: string | null;
  onSelectGroup: (groupId: string) => void;
}

/**
 * 顶部套餐分组 tabs：按计费周期（月付/季付/年付等）切换套餐列表。
 * 等宽紧凑布局，全部分组一屏可见，无需横向滚动。
 */
export const VipPlanTabs: React.FC<VipPlanTabsProps> = ({
  groups,
  activeGroupId,
  onSelectGroup,
}) => {
  if (groups.length <= 1) {
    return null;
  }

  return (
    <div className="flex bg-chat-other-bg border-b border-border-color shrink-0">
      {groups.map((group) => {
        const active = group.id === activeGroupId;
        return (
          <button
            key={group.id}
            type="button"
            onClick={() => onSelectGroup(group.id)}
            className={cn(
              "flex-1 min-w-0 px-1 py-2.5 text-center text-[13px] font-medium border-b-2 transition-colors active:opacity-80",
              active
                ? "text-primary-blue border-primary-blue"
                : "text-text-sub border-transparent",
            )}
          >
            <span className="block truncate">{group.name}</span>
          </button>
        );
      })}
    </div>
  );
};
