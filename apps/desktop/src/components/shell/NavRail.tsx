import React from 'react';
import {
  FolderGit2,
  Workflow,
  ListChecks,
  Smartphone,
  BugPlay,
  PackageCheck,
  Users,
  Network,
  Settings,
  ChevronLeft,
  ChevronRight,
  Terminal,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useProjectStore } from '../../stores/projectStore';

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
  const { projects, activeProjectDetail } = useProjectStore();

  const readyCount = devices.filter((d) => d.connection_state === 'ready' && !d.is_mock).length;
  const wfCount = activeProjectDetail?.workflows.length ?? 0;
  const taskCount = activeProjectDetail?.tasks.length ?? 0;
  const releaseCount = activeProjectDetail?.releases.length ?? 0;

  const navSections = [
    {
      title: 'STUDIO SẢN XUẤT',
      items: [
        {
          id: 'projects',
          label: 'Projects & Dashboard',
          icon: FolderGit2,
          badge: projects.length > 0 ? `${projects.length}` : null,
          badgeColor: 'bg-[#173355] text-[#4B9CFF]',
        },
        {
          id: 'studio',
          label: 'Workflow Studio',
          icon: Workflow,
          badge: wfCount > 0 ? `${wfCount} WF` : 'Canvas',
          badgeColor: 'bg-[#173355] text-[#38BDF8]',
        },
        {
          id: 'tasks',
          label: 'Tasks & Modules',
          icon: ListChecks,
          badge: taskCount > 0 ? `${taskCount}` : null,
          badgeColor: 'bg-[#202B3B] text-[#A9B8CC]',
        },
      ],
    },
    {
      title: 'THIẾT BỊ & KIỂM THỬ',
      items: [
        {
          id: 'devices',
          label: 'Device Manager',
          icon: Smartphone,
          badge: devices.length > 0 ? `${devices.length}` : '0',
          badgeColor: 'bg-[#202B3B] text-[#94A3B8]',
        },
        {
          id: 'device-studio',
          label: 'Device Studio',
          icon: Smartphone,
          badge: readyCount > 0 ? `${readyCount} LIVE` : 'Studio',
          badgeColor:
            readyCount > 0
              ? 'bg-[#133528] text-[#3BC38D]'
              : 'bg-[#172554] text-[#60A5FA]',
        },
        {
          id: 'runs',
          label: 'Test & Debug',
          icon: BugPlay,
          badge: isRunning ? 'RUN' : null,
          badgeColor: 'bg-[#3B2D17] text-[#E7B15B] animate-pulse',
        },
      ],
    },
    {
      title: 'ĐÓNG GÓI & PHÁT HÀNH',
      items: [
        {
          id: 'builder',
          label: 'Mini Tool Builder',
          icon: PackageCheck,
          badge: releaseCount > 0 ? `${releaseCount} EXE` : 'Build',
          badgeColor:
            releaseCount > 0
              ? 'bg-[#133528] text-[#3BC38D]'
              : 'bg-[#202B3B] text-[#B291F7]',
        },
      ],
    },
    {
      title: 'HẠ TẦNG & HỆ THỐNG',
      items: [
        { id: 'accounts', label: 'Account Vault', icon: Users, badge: null },
        { id: 'network', label: 'Proxy & Port Map', icon: Network, badge: null },
        { id: 'settings', label: 'System Settings', icon: Settings, badge: null },
      ],
    },
  ];

  const isViewMatch = (itemId: string) => {
    if (itemId === 'projects' && (currentView === 'overview' || currentView === 'workflows')) {
      return true;
    }
    return currentView === itemId;
  };

  return (
    <aside
      className={`bg-[#161E29] border-r border-[#2A3647] flex flex-col justify-between transition-all duration-200 select-none z-30 shrink-0 ${
        isExpanded ? 'w-[220px]' : 'w-[64px]'
      }`}
    >
      {/* Navigation Sections */}
      <div className="flex flex-col py-2.5 space-y-3.5 overflow-y-auto">
        {navSections.map((sec, idx) => (
          <div key={idx} className="flex flex-col space-y-0.5">
            {isExpanded && (
              <span className="px-3.5 pb-1 text-[10px] font-bold text-[#5A6D85] tracking-wider uppercase">
                {sec.title}
              </span>
            )}
            {sec.items.map((item) => {
              const Icon = item.icon;
              const isActive = isViewMatch(item.id);
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
                  {isActive && (
                    <div className="absolute left-0 top-1.5 bottom-1.5 w-1 bg-[#1878F5] rounded-r" />
                  )}

                  <Icon
                    size={16}
                    className={`shrink-0 ${
                      isActive ? 'text-[#4B9CFF]' : 'text-[#74869D] group-hover:text-[#A9B8CC]'
                    }`}
                  />

                  {isExpanded && (
                    <div className="ml-2.5 flex-1 flex items-center justify-between min-w-0">
                      <span className="text-[12.5px] truncate">{item.label}</span>
                      {item.badge && (
                        <span
                          className={`text-[9.5px] px-1.5 py-0.2 rounded font-mono font-semibold shrink-0 ml-1 ${
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

      {/* Bottom Footer: Environment & Collapse Toggle */}
      <div className="p-2 border-t border-[#2A3647] flex flex-col space-y-1.5">
        {isExpanded && (
          <div className="px-2.5 py-1.5 rounded bg-[#101722] border border-[#2A3647]/70 flex items-center justify-between text-[11px]">
            <div className="flex items-center gap-1.5 text-[#A9B8CC]">
              <Terminal size={12} className={readyCount > 0 ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
              <span className="font-mono text-[10.5px]">
                ADB {readyCount > 0 ? `${readyCount} online` : 'standby'}
              </span>
            </div>
            <span
              className={`w-2 h-2 rounded-full ${readyCount > 0 ? 'bg-[#3BC38D]' : 'bg-[#74869D]'}`}
              title={readyCount > 0 ? 'Có thiết bị ADB ready' : 'Chưa phát hiện thiết bị ADB ready'}
            />
          </div>
        )}

        <button
          onClick={onToggleExpand}
          className="w-full flex items-center justify-center p-1.5 rounded hover:bg-[#192230] text-[#74869D] hover:text-[#EDF3FC] transition-colors cursor-pointer"
          title={isExpanded ? 'Thu gọn thanh bên (64px)' : 'Mở rộng thanh bên (220px)'}
          aria-label="Toggle Sidebar"
        >
          {isExpanded ? (
            <div className="flex items-center gap-1.5 text-xs">
              <ChevronLeft size={15} />
              <span>Thu gọn Sidebar</span>
            </div>
          ) : (
            <ChevronRight size={16} />
          )}
        </button>
      </div>
    </aside>
  );
};
