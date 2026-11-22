import React, { useEffect, useState } from 'react';
import {
  FolderGit2,
  FolderPlus,
  Workflow,
  ListChecks,
  PackageCheck,
  Smartphone,
  Copy,
  Edit3,
  Trash2,
  Search,
  DatabaseBackup,
  ShieldCheck,
  Image,
  Layers,
  ArrowRight,
  FolderOpen,
  ShieldAlert,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api, isTauri } from '../../lib/api';
import type { GameProjectRecord, IntegrityReport } from '../../types';
import {
  CloneProjectModal,
  CreateProjectModal,
  DeleteProjectModal,
  EditProjectModal,
  LegacyMigrateModal,
} from './ProjectModals';

interface ProjectsDashboardViewProps {
  onNavigate: (view: string) => void;
}

export const ProjectsDashboardView: React.FC<ProjectsDashboardViewProps> = ({ onNavigate }) => {
  const {
    projects,
    activeProjectId,
    activeProjectDetail,
    selectProject,
    fetchProjects,
    createProject,
    updateProject,
    cloneProject,
    deleteProject,
    isLoadingProjects,
    isLoadingDetail,
    lastError,
  } = useProjectStore();
  const { devices } = useDeviceStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [cloneTarget, setCloneTarget] = useState<GameProjectRecord | null>(null);
  const [editTarget, setEditTarget] = useState<GameProjectRecord | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<GameProjectRecord | null>(null);
  const [showMigrate, setShowMigrate] = useState(false);
  const [integrity, setIntegrity] = useState<IntegrityReport | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (isTauri()) {
      api
        .studioCheckDbIntegrity()
        .then((rep) => setIntegrity(rep))
        .catch(() => {});
    }
  }, [projects.length]);

  const pushNotice = (msg: string) => {
    setNotice(msg);
    setTimeout(() => setNotice(null), 4000);
  };

  const filteredProjects = projects.filter((item) => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return true;
    const p = item.project;
    return (
      p.name.toLowerCase().includes(q) ||
      p.slug.toLowerCase().includes(q) ||
      p.gamePackageId.toLowerCase().includes(q) ||
      p.exeName.toLowerCase().includes(q)
    );
  });

  const activeSummary = projects.find((p) => p.project.id === activeProjectId);
  const activeProj = activeProjectDetail?.project || activeSummary?.project || null;
  const readyLiveDevices = devices.filter((d) => d.connection_state === 'ready' && !d.is_mock).length;
  const totalReleases = projects.reduce((acc, p) => acc + p.releasesCount, 0);

  return (
    <div className="flex-1 h-full overflow-y-auto bg-[#101722] p-4 lg:p-5 space-y-4">
      {/* Studio Software Factory Top Header Card */}
      <div className="fp-panel p-4 lg:p-5 flex flex-col xl:flex-row xl:items-center xl:justify-between gap-4">
        <div className="space-y-1.5">
          <div className="flex flex-wrap items-center gap-2">
            <span className="px-2 py-0.5 rounded text-[10.5px] font-bold bg-[#173355] text-[#4B9CFF] border border-[#2A3647]">
              MINI TOOL SOFTWARE FACTORY · P4.1 MULTI-PROJECT
            </span>
            {integrity && (
              <span
                className={`px-2 py-0.5 rounded text-[10.5px] font-semibold flex items-center gap-1 ${
                  integrity.isHealthy ? 'fp-badge-live' : 'fp-badge-error'
                }`}
              >
                <ShieldCheck size={11} />
                SQLite WAL {integrity.integrityCheck.toUpperCase()} (FK={integrity.foreignKeyViolations})
              </span>
            )}
            {!isTauri() && (
              <span className="px-2 py-0.5 rounded text-[10.5px] font-semibold fp-badge-testkit">
                WEB PREVIEW (Cần chạy FarmerPro.exe cho SQLite thật)
              </span>
            )}
          </div>
          <h1 className="text-[20px] lg:text-[22px] font-bold text-[#EDF3FC]">
            Game Projects &amp; Release Workspace
          </h1>
          <p className="text-[12.5px] text-[#A9B8CC] max-w-3xl">
            Quy trình sản xuất chuẩn IDE: Tạo/Clone Game Project → Thiết kế Workflow → Biên soạn Task
            &amp; Module → Test/Debug trên Device Lab → Thiết kế UI &amp; Build Mini Tool `.exe`.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2 shrink-0">
          <button
            onClick={() => setShowMigrate(true)}
            className="px-3 py-2 rounded-md bg-[#202B3B] hover:bg-[#2A3647] border border-[#2A3647] text-xs font-medium text-[#A9B8CC] hover:text-white flex items-center gap-1.5 cursor-pointer"
          >
            <DatabaseBackup size={14} className="text-[#E7B15B]" />
            <span>Import Legacy</span>
          </button>
          <button
            onClick={() => setShowCreate(true)}
            className="px-3.5 py-2 rounded-md bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold flex items-center gap-1.5 shadow-sm cursor-pointer"
          >
            <FolderPlus size={14} />
            <span>Tạo Game Project</span>
          </button>
        </div>
      </div>

      {notice && (
        <div className="p-2.5 rounded-md fp-badge-live text-xs font-medium flex items-center gap-2">
          <ShieldCheck size={14} />
          <span>{notice}</span>
        </div>
      )}

      {lastError && (
        <div className="p-2.5 rounded-md fp-badge-error text-xs flex items-center gap-2">
          <ShieldAlert size={14} />
          <span>{lastError}</span>
        </div>
      )}

      {/* Factory KPIs */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <div className="fp-panel p-3.5 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">Game Projects</div>
            <div className="text-xl font-bold text-[#EDF3FC] mt-0.5">{projects.length}</div>
            <div className="text-[11px] text-[#A9B8CC]">Cô lập SQLite + Folder</div>
          </div>
          <FolderGit2 size={22} className="text-[#4B9CFF]" />
        </div>

        <div className="fp-panel p-3.5 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">
              Active Workflows / Tasks
            </div>
            <div className="text-xl font-bold text-[#EDF3FC] mt-0.5">
              {activeProjectDetail?.workflows.length ?? 0} / {activeProjectDetail?.tasks.length ?? 0}
            </div>
            <div className="text-[11px] text-[#A9B8CC] truncate max-w-[140px]">
              {activeProj ? activeProj.slug : 'Chưa chọn'}
            </div>
          </div>
          <Workflow size={22} className="text-[#38BDF8]" />
        </div>

        <div className="fp-panel p-3.5 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">
              Bản Phát hành (Releases)
            </div>
            <div className="text-xl font-bold text-[#EDF3FC] mt-0.5">{totalReleases}</div>
            <div className="text-[11px] text-[#A9B8CC]">
              {activeProjectDetail?.releases[0]?.version
                ? `Mới nhất v${activeProjectDetail.releases[0].version}`
                : 'Chưa build EXE'}
            </div>
          </div>
          <PackageCheck size={22} className="text-[#3BC38D]" />
        </div>

        <div className="fp-panel p-3.5 flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-wider text-[#74869D]">Device Lab</div>
            <div className="text-xl font-bold text-[#EDF3FC] mt-0.5">
              {readyLiveDevices} / {devices.length}
            </div>
            <div className="text-[11px] text-[#A9B8CC]">
              {readyLiveDevices > 0 ? 'ADB LIVE sẵn sàng' : 'Chưa gắn thiết bị LIVE'}
            </div>
          </div>
          <Smartphone size={22} className={readyLiveDevices > 0 ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
        </div>
      </div>

      {/* Main Split: Project List (Left) + Isolated Resource Inspector (Right) */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-4 items-start">
        {/* Left: Project Catalog */}
        <div className="xl:col-span-7 fp-panel p-4 space-y-3">
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
            <h2 className="text-[15px] font-semibold text-[#EDF3FC]">
              Danh sách Projects ({filteredProjects.length})
            </h2>
            <div className="relative w-full sm:w-64">
              <Search size={14} className="absolute left-2.5 top-2.5 text-[#74869D]" />
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Tìm tên, slug, package, EXE..."
                className="w-full pl-8 pr-3 py-1.5 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none text-xs text-[#EDF3FC]"
              />
            </div>
          </div>

          {isLoadingProjects ? (
            <div className="p-8 text-center text-xs text-[#74869D]">
              Đang nạp danh sách Game Projects...
            </div>
          ) : filteredProjects.length === 0 ? (
            <div className="p-8 rounded-lg bg-[#121A26] border border-dashed border-[#2A3647] text-center space-y-3">
              <div className="text-sm font-medium text-[#EDF3FC]">
                {projects.length === 0
                  ? 'Chưa có Game Project nào trong SQLite'
                  : 'Không có Project khớp bộ lọc'}
              </div>
              <p className="text-xs text-[#74869D]">
                Nhấn <strong>Tạo Game Project</strong> để khởi tạo không gian làm việc cô lập mới, hoặc{' '}
                <strong>Import Legacy</strong> để nhập dự án mẫu.
              </p>
              <button
                onClick={() => setShowCreate(true)}
                className="px-3.5 py-1.5 rounded bg-[#1878F5] text-white text-xs font-semibold cursor-pointer inline-flex items-center gap-1.5"
              >
                <FolderPlus size={13} />
                <span>Tạo Project đầu tiên</span>
              </button>
            </div>
          ) : (
            <div className="space-y-2.5">
              {filteredProjects.map((item) => {
                const p = item.project;
                const isSelected = p.id === activeProjectId;
                return (
                  <div
                    key={p.id}
                    onClick={() => selectProject(p.id)}
                    className={`p-3.5 rounded-lg border transition-all cursor-pointer ${
                      isSelected
                        ? 'bg-[#173355]/55 border-[#4B9CFF]'
                        : 'bg-[#161E29] border-[#2A3647] hover:border-[#3B4B61]'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0 space-y-1">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="font-semibold text-[13.5px] text-[#EDF3FC] truncate">
                            {p.name}
                          </span>
                          <span className="text-[10.5px] font-mono px-1.5 py-0.2 rounded bg-[#202B3B] text-[#4B9CFF]">
                            v{p.version}
                          </span>
                          {isSelected && (
                            <span className="text-[10px] px-1.5 py-0.2 rounded fp-badge-live font-semibold">
                              ACTIVE
                            </span>
                          )}
                        </div>
                        <div className="text-[11px] font-mono text-[#74869D] truncate">
                          {p.slug} · {p.gamePackageId} · {p.targetWidthPx}×{p.targetHeightPx}
                        </div>
                        <div className="flex flex-wrap items-center gap-3 text-[11.5px] text-[#A9B8CC] pt-0.5">
                          <span className="flex items-center gap-1">
                            <Workflow size={12} className="text-[#38BDF8]" /> {item.workflowsCount} WF
                          </span>
                          <span className="flex items-center gap-1">
                            <ListChecks size={12} className="text-[#E7B15B]" /> {item.tasksCount} Tasks
                          </span>
                          <span className="flex items-center gap-1">
                            <Image size={12} className="text-[#B291F7]" /> {item.templatesCount} ROI
                          </span>
                          <span className="flex items-center gap-1">
                            <PackageCheck size={12} className="text-[#3BC38D]" /> {item.releasesCount} Rel
                          </span>
                        </div>
                      </div>

                      {/* CRUD Action Buttons */}
                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setCloneTarget(p);
                          }}
                          title="Deep Clone thành Project mới"
                          className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#3BC38D] cursor-pointer"
                        >
                          <Copy size={13} />
                        </button>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setEditTarget(p);
                          }}
                          title="Chỉnh sửa Metadata"
                          className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#4B9CFF] cursor-pointer"
                        >
                          <Edit3 size={13} />
                        </button>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setDeleteTarget(p);
                          }}
                          title="Xóa Project"
                          className="p-1.5 rounded bg-[#202B3B] hover:bg-[#3B1717] text-[#F27883] cursor-pointer"
                        >
                          <Trash2 size={13} />
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Right: Active Project Isolated Resource Inspector */}
        <div className="xl:col-span-5 fp-panel p-4 space-y-3.5">
          <div className="flex items-center justify-between border-b border-[#2A3647] pb-2.5">
            <div>
              <h2 className="text-[15px] font-semibold text-[#EDF3FC]">
                Tài nguyên &amp; Cô lập (Project Detail)
              </h2>
              <p className="text-[11px] text-[#74869D]">
                Dữ liệu gắn chặt vào khóa ngoại <code>project_id</code> trong SQLite
              </p>
            </div>
            {activeProj && (
              <span className="text-[10.5px] font-mono px-2 py-0.5 rounded bg-[#101722] border border-[#2A3647] text-[#A9B8CC]">
                {activeProj.id}
              </span>
            )}
          </div>

          {!activeProj ? (
            <div className="p-6 text-center text-xs text-[#74869D]">
              Chọn một Game Project bên trái để kiểm tra workflows, templates, modules và releases
              thuộc dự án đó.
            </div>
          ) : isLoadingDetail ? (
            <div className="p-6 text-center text-xs text-[#74869D]">Đang tải chi tiết dự án...</div>
          ) : (
            <div className="space-y-3 text-xs">
              <div className="p-3 rounded bg-[#101722] border border-[#2A3647] space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-[#74869D]">Đầu ra EXE:</span>
                  <span className="font-mono font-semibold text-[#EDF3FC]">
                    {activeProj.exeName}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[#74869D]">ADB Bundling:</span>
                  <span className="text-[#A9B8CC]">
                    {activeProj.bundleAdb ? 'Có (Portable bin/)' : 'Không'}
                  </span>
                </div>
                <div className="flex items-start gap-1.5 pt-1 text-[11px] font-mono text-[#74869D] break-all">
                  <FolderOpen size={13} className="shrink-0 mt-0.5 text-[#4B9CFF]" />
                  <span>{activeProj.projectDirPath}</span>
                </div>
              </div>

              {/* Isolated Counts & Quick Jump */}
              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={() => onNavigate('studio')}
                  className="p-2.5 rounded bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] flex items-center justify-between cursor-pointer"
                >
                  <div className="text-left">
                    <div className="text-[11px] text-[#74869D]">Workflows</div>
                    <div className="font-bold text-[#EDF3FC]">
                      {activeProjectDetail?.workflows.length ?? 0} luồng
                    </div>
                  </div>
                  <ArrowRight size={13} className="text-[#4B9CFF]" />
                </button>

                <button
                  onClick={() => onNavigate('tasks')}
                  className="p-2.5 rounded bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] flex items-center justify-between cursor-pointer"
                >
                  <div className="text-left">
                    <div className="text-[11px] text-[#74869D]">Tasks &amp; Modules</div>
                    <div className="font-bold text-[#EDF3FC]">
                      {activeProjectDetail?.tasks.length ?? 0} tasks ·{' '}
                      {activeProjectDetail?.modules.length ?? 0} mod
                    </div>
                  </div>
                  <ArrowRight size={13} className="text-[#E7B15B]" />
                </button>

                <button
                  onClick={() => onNavigate('devices')}
                  className="p-2.5 rounded bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] flex items-center justify-between cursor-pointer"
                >
                  <div className="text-left">
                    <div className="text-[11px] text-[#74869D]">ROI Templates</div>
                    <div className="font-bold text-[#EDF3FC]">
                      {activeProjectDetail?.templates.length ?? 0} ảnh PNG
                    </div>
                  </div>
                  <ArrowRight size={13} className="text-[#B291F7]" />
                </button>

                <button
                  onClick={() => onNavigate('builder')}
                  className="p-2.5 rounded bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] flex items-center justify-between cursor-pointer"
                >
                  <div className="text-left">
                    <div className="text-[11px] text-[#74869D]">Releases</div>
                    <div className="font-bold text-[#EDF3FC]">
                      {activeProjectDetail?.releases.length ?? 0} bản build
                    </div>
                  </div>
                  <ArrowRight size={13} className="text-[#3BC38D]" />
                </button>
              </div>

              {/* Packaged Modules Tags */}
              <div className="space-y-1.5">
                <div className="text-[11px] text-[#74869D] font-medium flex items-center gap-1">
                  <Layers size={12} />
                  <span>Modules đã cấu hình ({activeProjectDetail?.modules.length ?? 0})</span>
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {(activeProjectDetail?.modules || []).map((m) => (
                    <span
                      key={m.moduleId}
                      className={`px-2 py-0.5 rounded text-[10.5px] font-mono border ${
                        m.packagedInRelease
                          ? 'bg-[#173355]/70 border-[#4B9CFF]/50 text-[#EDF3FC]'
                          : 'bg-[#101722] border-[#2A3647] text-[#74869D]'
                      }`}
                    >
                      {m.moduleId}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Modals */}
      <CreateProjectModal
        isOpen={showCreate}
        onClose={() => setShowCreate(false)}
        onSubmit={async (req) => {
          const rec = await createProject(req);
          pushNotice(`Đã tạo Game Project '${rec.name}' (${rec.slug}) vào SQLite.`);
        }}
      />

      <CloneProjectModal
        key={cloneTarget?.id || 'clone-none'}
        sourceProject={cloneTarget}
        onClose={() => setCloneTarget(null)}
        onSubmit={async (req) => {
          const rec = await cloneProject(req);
          pushNotice(`Đã Deep Clone thành '${rec.name}' (${rec.slug}) độc lập.`);
        }}
      />

      <EditProjectModal
        key={editTarget?.id || 'edit-none'}
        project={editTarget}
        onClose={() => setEditTarget(null)}
        onSubmit={async (req) => {
          const rec = await updateProject(req);
          pushNotice(`Đã cập nhật metadata cho '${rec.name}' (v${rec.version}).`);
        }}
      />

      <DeleteProjectModal
        project={deleteTarget}
        onClose={() => setDeleteTarget(null)}
        onConfirm={async (projectId, removeDisk) => {
          await deleteProject(projectId, removeDisk);
          pushNotice(`Đã xóa project khỏi SQLite.`);
        }}
      />

      <LegacyMigrateModal
        isOpen={showMigrate}
        onClose={() => setShowMigrate(false)}
        onMigrate={async (legacyDir) => {
          const res = await api.studioMigrateLegacyJx2(legacyDir);
          await fetchProjects(true);
          pushNotice(
            `Đã import '${res.projectSlug}': ${res.workflowsImported} workflows, ${res.templatesImported} templates (bản gốc giữ nguyên).`
          );
        }}
      />
    </div>
  );
};
