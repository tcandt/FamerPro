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
} from 'lucide-react';

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
  const navSections = [
    {
      title: 'VẬN HÀNH',
      items: [
        { id: 'overview', label: 'Tổng quan', icon: LayoutDashboard },
        { id: 'devices', label: 'Thiết bị', icon: Smartphone },
        { id: 'studio', label: 'Workflow Studio', icon: Workflow },
        { id: 'tasks', label: 'Nhiệm vụ', icon: ListChecks },
        { id: 'runs', label: 'Phiên chạy', icon: Activity },
      ],
    },
    {
      title: 'TÀI NGUYÊN',
      items: [
        { id: 'accounts', label: 'Tài khoản', icon: Users },
        { id: 'network', label: 'Mạng / Proxy', icon: Network },
      ],
    },
    {
      title: 'HỆ THỐNG',
      items: [
        { id: 'gallery', label: 'Design Gallery', icon: Palette },
        { id: 'settings', label: 'Cài đặt', icon: Settings },
      ],
    },
  ];

  return (
    <aside
      className={`bg-[#161E29] border-r border-[#2A3647] flex flex-col justify-between transition-all duration-200 select-none z-30 ${
        isExpanded ? 'w-[220px]' : 'w-[64px]'
      }`}
    >
      <div className="flex flex-col py-3 space-y-4">
        {navSections.map((sec, idx) => (
          <div key={idx} className="flex flex-col space-y-1">
            {isExpanded && (
              <span className="px-4 text-[10px] font-semibold text-[#74869D] tracking-wider uppercase">
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
                  className={`flex items-center h-9 px-3 mx-2 rounded relative group cursor-pointer transition-colors ${
                    isActive
                      ? 'bg-[#173355] text-[#EDF3FC]'
                      : 'text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#192230]'
                  }`}
                >
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
                    <span className="ml-3 text-xs font-medium truncate">{item.label}</span>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </div>

      {/* Collapse / Expand Toggle */}
      <div className="p-3 border-t border-[#2A3647] flex items-center justify-end">
        <button
          onClick={onToggleExpand}
          className="p-1.5 rounded hover:bg-[#192230] text-[#74869D] hover:text-[#EDF3FC] transition-colors"
          title={isExpanded ? 'Thu gọn thanh bên' : 'Mở rộng thanh bên'}
        >
          {isExpanded ? <ChevronLeft size={16} /> : <ChevronRight size={16} />}
        </button>
      </div>
    </aside>
  );
};
