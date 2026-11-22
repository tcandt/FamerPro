import React from 'react';
import { PackageCheck, FolderGit2, ShieldCheck, Wrench } from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';

interface MiniToolBuilderViewProps {
  onNavigate?: (view: string) => void;
}

export const MiniToolBuilderView: React.FC<MiniToolBuilderViewProps> = ({ onNavigate }) => {
  const { activeProjectDetail } = useProjectStore();
  const project = activeProjectDetail?.project;
  const releases = activeProjectDetail?.releases || [];

  return (
    <div className="flex-1 h-full overflow-y-auto bg-[#101722] p-5 space-y-4">
      <div className="fp-panel p-5 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded text-[11px] font-semibold bg-[#173355] text-[#B291F7] border border-[#2A3647]">
              MINI TOOL BUILDER &amp; RELEASES · KHUNG P4.0 (WYSIWYG Ở P4.5)
            </span>
          </div>
          <h1 className="text-[20px] font-bold text-[#EDF3FC]">
            Thiết kế Giao diện Mini Tool &amp; Đóng gói Phát hành
          </h1>
          <p className="text-xs text-[#A9B8CC]">
            {project
              ? `Đang gắn ngữ cảnh dự án: ${project.name} (${project.exeName} · v${project.version})`
              : 'Chọn một Game Project từ Header để thiết kế giao diện Mini Tool và xuất EXE ký Ed25519.'}
          </p>
        </div>

        {onNavigate && (
          <button
            onClick={() => onNavigate('projects')}
            className="px-3 py-2 rounded-md bg-[#202B3B] hover:bg-[#2A3647] border border-[#2A3647] text-xs text-[#EDF3FC] flex items-center gap-1.5 cursor-pointer self-start"
          >
            <FolderGit2 size={14} className="text-[#4B9CFF]" />
            <span>Quản lý Projects</span>
          </button>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div className="fp-panel p-4 space-y-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-[#EDF3FC]">
            <Wrench size={16} className="text-[#4B9CFF]" />
            <span>Sẵn sàng tích hợp WYSIWYG Builder (Milestone P4.5)</span>
          </div>
          <p className="text-xs text-[#A9B8CC] leading-relaxed">
            Khu vực này sẽ cung cấp Component Palette, Live Preview các trạng thái runtime của
            Mini Tool, cấu hình Module Matrix, Icon/Branding và nút Preflight + Build EXE nối trực
            tiếp vào IPC <code>studio_builder_preflight</code> và{' '}
            <code>studio_builder_export_release</code>.
          </p>
        </div>

        <div className="fp-panel p-4 space-y-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-[#EDF3FC]">
            <PackageCheck size={16} className="text-[#3BC38D]" />
            <span>Lịch sử Phát hành Dự án ({releases.length})</span>
          </div>
          {releases.length === 0 ? (
            <p className="text-xs text-[#74869D]">
              Chưa có bản phát hành nào được ghi nhận trong <code>project_releases</code> cho dự án này.
            </p>
          ) : (
            <div className="space-y-1.5">
              {releases.map((rel) => (
                <div
                  key={rel.id}
                  className="p-2.5 rounded bg-[#161E29] border border-[#2A3647] flex items-center justify-between text-xs"
                >
                  <div className="min-w-0">
                    <div className="font-semibold text-[#EDF3FC] truncate">
                      v{rel.version} · {rel.exePath}
                    </div>
                    <div className="font-mono text-[11px] text-[#74869D] truncate">
                      Key {rel.keyId} · SHA {rel.exeSha256.slice(0, 16)}...
                    </div>
                  </div>
                  <ShieldCheck size={15} className="text-[#3BC38D] shrink-0 ml-2" />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
