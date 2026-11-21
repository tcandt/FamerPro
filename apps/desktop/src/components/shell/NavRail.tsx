import React from 'react';
import {
  LayoutDashboard,
  Smartphone,
  Workflow,
  ListChecks,
  Activity,
  Users,
  Network,
  Settings,
  Palette,
  ChevronLeft,
  ChevronRight,
  Terminal,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';

interface NavRailProps {
  currentView: string;
  onSelectView: (view: string) => void;
  isExpanded: boolean;
  onToggleExpand: () => void;
}

export const NavRail: React.FC<NavRailProps> = ({
  currentView,
  onSelectView,
  isExpanded,
  onToggleExpand,
}) => {
  const { devices } = useDeviceStore();
  const { isRunning } = useWorkflowStore();
  const readyCount = devices.filter((d) => d.connection_state === 'ready').length;

  const navSections = [
    {
      title: 'VẬN HÀNH CHÍNH',
      items: [
        { id: 'overview', label: 'Tổng quan', icon: LayoutDashboard, badge: null },
        {
          id: 'devices',
          label: 'Thiết bị Android',
          icon: Smartphone,
          badge: readyCount > 0 ? `${readyCount} online` : `${devices.length}`,
          badgeColor: readyCount > 0 ? 'bg-[#133528] text-[#3BC38D]' : 'bg-[#202B3B] text-[#74869D]',
        },
        { id: 'studio', label: 'Workflow Studio', icon: Workflow, badge: 'Canvas' },
        { id: 'tasks', label: 'Bộ Nhiệm vụ', icon: ListChecks, badge: 'Queue' },
        {
          id: 'runs',
          label: 'Phiên chạy',
          icon: Activity,
          badge: isRunning ? 'Đang chạy' : null,
          badgeColor: 'bg-[#3B2D17] text-[#E7B15B] animate-pulse',
        },
      ],
    },
    {
      title: 'TÀI NGUYÊN & MẠNG',
      items: [
        { id: 'accounts', label: 'Tài khoản (Vault)', icon: Users, badge: null },
        { id: 'network', label: 'Mạng & Proxy', icon: Network, badge: null },
      ],
    },
    {
      title: 'HỆ THỐNG',
      items: [
        { id: 'gallery', label: 'Design Tokens', icon: Palette, badge: 'Tokens' },
        { id: 'settings', label: 'Cài đặt & ADB', icon: Settings, badge: null },
      ],
    },
  ];

  return (
    <aside
      className={`bg-[#141C28] border-r border-[#2A3647] flex flex-col justify-between transition-all duration-200 select-none z-30 shrink-0 ${
        isExpanded ? 'w-[230px]' : 'w-[64px]'
      }`}
    >
      {/* Top Nav Items */}
      <div className="flex flex-col py-3 space-y-4 overflow-y-auto">
        {navSections.map((sec, idx) => (
          <div key={idx} className="flex flex-col space-y-1">
            {isExpanded && (
              <span className="px-4 text-[10px] font-bold text-[#5A6D85] tracking-wider uppercase">
                {sec.title}
              </span>
            )}
            {sec.items.map((item) => {
              const Icon = item.icon;
              const isActive = currentView === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => onSelectView(item.id)}
                  title={!isExpanded ? item.label : undefined}
                  className={`flex items-center h-9 px-3 mx-2 rounded-md relative group cursor-pointer transition-all ${
                    isActive
                      ? 'bg-[#173355] text-white font-medium shadow-sm'
                      : 'text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#192230]'
                  }`}
                >
                  {/* Left Active Indicator Bar */}
                  {isActive && (
                    <div className="absolute left-0 top-1.5 bottom-1.5 w-1 bg-[#1878F5] rounded-r" />
                  )}

                  <Icon
                    size={17}
                    className={`shrink-0 ${
                      isActive ? 'text-[#1878F5]' : 'text-[#74869D] group-hover:text-[#A9B8CC]'
                    }`}
                  />

                  {isExpanded && (
                    <div className="ml-3 flex-1 flex items-center justify-between min-w-0">
                      <span className="text-xs truncate">{item.label}</span>
                      {item.badge && (
                        <span
                          className={`text-[9px] px-1.5 py-0.2 rounded font-mono font-medium ${
                            item.badgeColor || 'bg-[#202B3B] text-[#7BA6D6]'
                          }`}
                        >
                          {item.badge}
                        </span>
                      )}
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </div>

      {/* Bottom Footer: Environment Health & Expand Toggle */}
      <div className="p-2 border-t border-[#2A3647] flex flex-col space-y-2">
        {isExpanded && (
          <div className="px-2 py-1.5 rounded bg-[#101722] border border-[#2A3647]/60 flex items-center justify-between text-[11px]">
            <div className="flex items-center gap-1.5 text-[#3BC38D]">
              <Terminal size={12} />
              <span className="font-mono text-[10px]">Portable ADB</span>
            </div>
            <span className="w-2 h-2 rounded-full bg-[#3BC38D]" title="ADB Ready" />
          </div>
        )}

        <button
          onClick={onToggleExpand}
          className="w-full flex items-center justify-center p-1.5 rounded hover:bg-[#192230] text-[#74869D] hover:text-[#EDF3FC] transition-colors cursor-pointer"
          title={isExpanded ? 'Thu gọn thanh bên (64px)' : 'Mở rộng thanh bên (230px)'}
        >
          {isExpanded ? (
            <div className="flex items-center gap-2 text-xs">
              <ChevronLeft size={15} />
              <span>Thu gọn thanh bên</span>
            </div>
          ) : (
            <ChevronRight size={16} />
          )}
        </button>
      </div>
    </aside>
  );
};
