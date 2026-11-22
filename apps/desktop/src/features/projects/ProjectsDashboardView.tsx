import React from 'react';
import {
  FolderGit2,
  Workflow,
  ListChecks,
  PackageCheck,
  Smartphone,
  ArrowRight,
  Layers,
  Image,
  ShieldAlert,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { isTauri } from '../../lib/api';

interface ProjectsDashboardViewProps {
  onNavigate: (view: string) => void;
}

export const ProjectsDashboardView: React.FC<ProjectsDashboardViewProps> = ({ onNavigate }) => {
  const {
    projects,
    activeProjectId,
    activeProjectDetail,
    selectProject,
    isLoadingProjects,
    lastError,
  } = useProjectStore();
  const { devices } = useDeviceStore();

  const activeProj =
    activeProjectDetail?.project || projects.find((p) => p.project.id === activeProjectId)?.project;
  const readyCount = devices.filter((d) => d.connection_state === 'ready' && !d.is_mock).length;

  return (
    <div className="flex-1 h-full overflow-y-auto bg-[#101722] p-5 space-y-5">
      {/* Top IDE Banner */}
      <div className="fp-panel p-5 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded text-[11px] font-semibold bg-[#173355] text-[#4B9CFF] border border-[#2A3647]">
              SOFTWARE FACTORY DASHBOARD · P4.0 FOUNDATION
            </span>
            {!isTauri() && (
              <span className="px-2 py-0.5 rounded text-[11px] font-semibold fp-badge-testkit">
                WEB PREVIEW (KHÔNG CÓ SQLITE TAURI)
              </span>
            )}
          </div>
          <h1 className="text-[22px] font-bold text-[#EDF3FC]">
            FarmerPro Multi-Project Auto Tool Studio
          </h1>
          <p className="text-[13px] text-[#A9B8CC] max-w-3xl">
            Quản lý các dự án Mini Auto-Tool độc lập theo từng game: biên soạn Workflow, ánh xạ
            Task &amp; Module, chụp ROI từ Device Lab, kiểm thử Runtime và đóng gói bản phát hành.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2 shrink-0">
          <button
            onClick={() => onNavigate('studio')}
            className="px-3.5 py-2 rounded-md bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
          >
            <Workflow size={14} />
            <span>Mở Workflow Studio</span>
          </button>
          <button
            onClick={() => onNavigate('builder')}
            className="px-3.5 py-2 rounded-md bg-[#202B3B] hover:bg-[#2A3647] border border-[#2A3647] text-[#EDF3FC] text-xs font-medium flex items-center gap-1.5 cursor-pointer"
          >
            <PackageCheck size={14} className="text-[#3BC38D]" />
            <span>Mini Tool Builder</span>
          </button>
        </div>
      </div>

      {lastError && (
        <div className="p-3 rounded-md fp-badge-error flex items-center gap-2 text-xs">
          <ShieldAlert size={15} />
          <span>Lỗi nạp Project SQLite: {lastError}</span>
        </div>
      )}

      {/* KPI Row */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3.5">
        <div className="fp-panel p-4 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">Tổng số Projects</div>
            <div className="text-2xl font-bold text-[#EDF3FC] mt-1">{projects.length}</div>
            <div className="text-[11px] text-[#A9B8CC] mt-0.5">Cô lập trong SQLite &amp; disk</div>
          </div>
          <FolderGit2 size={24} className="text-[#4B9CFF]" />
        </div>

        <div className="fp-panel p-4 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">
              Workflows (Active Project)
            </div>
            <div className="text-2xl font-bold text-[#EDF3FC] mt-1">
              {activeProjectDetail?.workflows.length ?? 0}
            </div>
            <div className="text-[11px] text-[#A9B8CC] mt-0.5">
              {activeProj ? activeProj.slug : 'Chưa chọn project'}
            </div>
          </div>
          <Workflow size={24} className="text-[#38BDF8]" />
        </div>

        <div className="fp-panel p-4 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">
              Tasks &amp; Templates
            </div>
            <div className="text-2xl font-bold text-[#EDF3FC] mt-1">
              {activeProjectDetail?.tasks.length ?? 0} / {activeProjectDetail?.templates.length ?? 0}
            </div>
            <div className="text-[11px] text-[#A9B8CC] mt-0.5">Nhiệm vụ &amp; ảnh ROI</div>
          </div>
          <Image size={24} className="text-[#B291F7]" />
        </div>

        <div className="fp-panel p-4 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">
              Thiết bị Device Lab
            </div>
            <div className="text-2xl font-bold text-[#EDF3FC] mt-1">
              {readyCount} / {devices.length}
            </div>
            <div className="text-[11px] text-[#A9B8CC] mt-0.5">
              {readyCount > 0 ? 'ADB LIVE sẵn sàng' : 'Chưa có thiết bị LIVE'}
            </div>
          </div>
          <Smartphone size={24} className={readyCount > 0 ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
        </div>
      </div>

      {/* Project Workspace Cards */}
      <div className="fp-panel p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-[15px] font-semibold text-[#EDF3FC]">
              Danh sách Game Projects (SQLite Multi-Project)
            </h2>
            <p className="text-xs text-[#74869D]">
              Chọn một Project làm ngữ cảnh toàn cục trên Header để biên soạn Workflow, Task và Build Mini Tool. (CRUD/Clone đầy đủ triển khai ở Milestone P4.1)
            </p>
          </div>
        </div>

        {isLoadingProjects ? (
          <div className="p-6 text-center text-xs text-[#74869D]">Đang tải danh sách dự án từ SQLite...</div>
        ) : projects.length === 0 ? (
          <div className="p-6 rounded-md bg-[#121A26] border border-dashed border-[#2A3647] text-center space-y-2">
            <div className="text-sm font-medium text-[#EDF3FC]">
              {isTauri()
                ? 'Chưa có Game Project nào trong cơ sở dữ liệu.'
                : 'Đang mở giao diện ngoài Tauri Runtime.'}
            </div>
            <p className="text-xs text-[#74869D]">
              Chức năng tạo/clone/import dự án sẽ kết nối trực tiếp vào bảng <code>projects</code> trong Milestone P4.1.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
            {projects.map((item) => {
              const p = item.project;
              const isSelected = p.id === activeProjectId;
              return (
                <div
                  key={p.id}
                  onClick={() => selectProject(p.id)}
                  className={`p-4 rounded-lg border transition-all cursor-pointer flex flex-col justify-between gap-3 ${
                    isSelected
                      ? 'bg-[#173355]/50 border-[#4B9CFF]'
                      : 'bg-[#161E29] border-[#2A3647] hover:border-[#3B4B61]'
                  }`}
                >
                  <div className="space-y-1.5">
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-semibold text-sm text-[#EDF3FC] truncate">{p.name}</span>
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#202B3B] text-[#4B9CFF]">
                        v{p.version}
                      </span>
                    </div>
                    <div className="text-[11px] font-mono text-[#74869D] truncate">
                      {p.slug} · {p.gamePackageId}
                    </div>
                    <div className="text-[11px] text-[#A9B8CC] flex items-center gap-3 pt-1">
                      <span className="flex items-center gap-1">
                        <Workflow size={12} /> {item.workflowsCount} WF
                      </span>
                      <span className="flex items-center gap-1">
                        <ListChecks size={12} /> {item.tasksCount} Tasks
                      </span>
                      <span className="flex items-center gap-1">
                        <Layers size={12} /> {item.releasesCount} Rel
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-[#2A3647]/70 text-xs">
                    <span className="font-mono text-[11px] text-[#74869D] truncate">{p.exeName}</span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        selectProject(p.id).then(() => onNavigate('studio'));
                      }}
                      className="text-[#4B9CFF] hover:text-white flex items-center gap-1 font-medium cursor-pointer shrink-0"
                    >
                      <span>Mở Studio</span>
                      <ArrowRight size={12} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
