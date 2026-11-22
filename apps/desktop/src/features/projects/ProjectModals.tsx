import React, { useState } from 'react';
import { X, FolderPlus, Copy, Edit3, Trash2, AlertTriangle, DatabaseBackup } from 'lucide-react';
import type {
  CloneProjectRequest,
  CreateProjectRequest,
  GameProjectRecord,
  UpdateProjectRequest,
} from '../../types';

const MODULE_OPTIONS = [
  { id: 'mod.adb_core', label: 'ADB Core Driver', locked: true },
  { id: 'mod.vision_ocr', label: 'Vision & OCR Engine', locked: true },
  { id: 'mod.account_vault', label: 'Account Vault (DPAPI)', locked: false },
  { id: 'mod.proxy_network', label: 'Proxy & Network Probe', locked: false },
  { id: 'mod.adb_port_forward', label: 'ADB Forward/Reverse', locked: false },
  { id: 'mod.emulator_optimizer', label: 'Emulator Optimizer', locked: false },
];

const RESOLUTION_PRESETS = [
  { label: 'Portrait 1080×1920', w: 1080, h: 1920 },
  { label: 'Landscape 1280×720', w: 1280, h: 720 },
  { label: 'Portrait 720×1280', w: 720, h: 1280 },
  { label: 'Landscape 1920×1080', w: 1920, h: 1080 },
];

function toSlug(name: string): string {
  return name
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40);
}

interface CreateProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (req: CreateProjectRequest) => Promise<void>;
}

export const CreateProjectModal: React.FC<CreateProjectModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
}) => {
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [slugTouched, setSlugTouched] = useState(false);
  const [pkg, setPkg] = useState('');
  const [version, setVersion] = useState('1.0.0');
  const [exeName, setExeName] = useState('');
  const [windowTitle, setWindowTitle] = useState('');
  const [widthPx, setWidthPx] = useState(1080);
  const [heightPx, setHeightPx] = useState(1920);
  const [bundleAdb, setBundleAdb] = useState(true);
  const [modules, setModules] = useState<string[]>([
    'mod.adb_core',
    'mod.vision_ocr',
    'mod.account_vault',
    'mod.proxy_network',
  ]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleNameChange = (val: string) => {
    setName(val);
    if (!slugTouched) {
      const nextSlug = toSlug(val);
      setSlug(nextSlug);
    }
  };

  const toggleModule = (modId: string, locked: boolean) => {
    if (locked) return;
    setModules((prev) =>
      prev.includes(modId) ? prev.filter((m) => m !== modId) : [...prev, modId]
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!name.trim() || !slug.trim() || !pkg.trim()) {
      setError('Vui lòng nhập đầy đủ Tên Project, Slug và Package ID.');
      return;
    }
    setSubmitting(true);
    try {
      await onSubmit({
        name: name.trim(),
        slug: slug.trim(),
        gamePackageId: pkg.trim(),
        version: version.trim() || '1.0.0',
        exeName: exeName.trim() || null,
        windowTitle: windowTitle.trim() || null,
        targetWidthPx: widthPx,
        targetHeightPx: heightPx,
        bundleAdb,
        enabledModules: modules,
      });
      setName('');
      setSlug('');
      setSlugTouched(false);
      setPkg('');
      onClose();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/65 flex items-center justify-center p-4">
      <div className="w-full max-w-xl bg-[#192230] border border-[#2A3647] rounded-xl shadow-2xl overflow-hidden">
        <div className="px-5 py-3.5 bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between">
          <div className="flex items-center gap-2 font-semibold text-sm text-[#EDF3FC]">
            <FolderPlus size={16} className="text-[#4B9CFF]" />
            <span>Tạo Game Project Mới (SQLite Multi-Project)</span>
          </div>
          <button
            onClick={onClose}
            className="text-[#74869D] hover:text-white cursor-pointer"
            aria-label="Close Create Modal"
          >
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-3.5 text-xs">
          {error && <div className="p-2.5 rounded fp-badge-error">{error}</div>}

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Tên Project *</span>
              <input
                value={name}
                onChange={(e) => handleNameChange(e.target.value)}
                placeholder="VD: Kingdom Siege Auto"
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Slug (thư mục cô lập) *</span>
              <input
                value={slug}
                onChange={(e) => {
                  setSlugTouched(true);
                  setSlug(e.target.value);
                }}
                placeholder="kingdom-siege"
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none font-mono text-[#EDF3FC]"
              />
            </label>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <label className="space-y-1 block sm:col-span-2">
              <span className="text-[#A9B8CC] font-medium">Android Package ID *</span>
              <input
                value={pkg}
                onChange={(e) => setPkg(e.target.value)}
                placeholder="com.studio.kingdomsiege"
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none font-mono text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Phiên bản</span>
              <input
                value={version}
                onChange={(e) => setVersion(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none font-mono text-[#EDF3FC]"
              />
            </label>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Tên EXE (tùy chọn)</span>
              <input
                value={exeName}
                onChange={(e) => setExeName(e.target.value)}
                placeholder="Auto_Kingdom_Siege.exe"
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none font-mono text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Tiêu đề cửa sổ Mini Tool</span>
              <input
                value={windowTitle}
                onChange={(e) => setWindowTitle(e.target.value)}
                placeholder="Kingdom Siege Auto v1.0"
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none text-[#EDF3FC]"
              />
            </label>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 items-end">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC] font-medium">Độ phân giải chuẩn</span>
              <select
                value={`${widthPx}x${heightPx}`}
                onChange={(e) => {
                  const [w, h] = e.target.value.split('x').map(Number);
                  setWidthPx(w);
                  setHeightPx(h);
                }}
                className="w-full px-2.5 py-2 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC]"
              >
                {RESOLUTION_PRESETS.map((p) => (
                  <option key={p.label} value={`${p.w}x${p.h}`}>
                    {p.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-2 py-2 px-3 rounded bg-[#101722] border border-[#2A3647] cursor-pointer sm:col-span-2">
              <input
                type="checkbox"
                checked={bundleAdb}
                onChange={(e) => setBundleAdb(e.target.checked)}
                className="accent-[#1878F5]"
              />
              <span className="text-[#EDF3FC]">Đóng gói Portable ADB khi xuất Mini Tool</span>
            </label>
          </div>

          <div className="space-y-1.5">
            <span className="text-[#A9B8CC] font-medium block">Module mặc định</span>
            <div className="grid grid-cols-2 gap-2">
              {MODULE_OPTIONS.map((m) => {
                const checked = modules.includes(m.id);
                return (
                  <label
                    key={m.id}
                    className={`flex items-center gap-2 px-2.5 py-1.5 rounded border text-[11.5px] ${
                      checked
                        ? 'bg-[#173355]/60 border-[#4B9CFF]/60 text-[#EDF3FC]'
                        : 'bg-[#101722] border-[#2A3647] text-[#A9B8CC]'
                    } ${m.locked ? 'opacity-80 cursor-not-allowed' : 'cursor-pointer'}`}
                  >
                    <input
                      type="checkbox"
                      checked={checked}
                      disabled={m.locked}
                      onChange={() => toggleModule(m.id, m.locked)}
                      className="accent-[#1878F5]"
                    />
                    <span className="truncate">{m.label}</span>
                  </label>
                );
              })}
            </div>
          </div>

          <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#A9B8CC] cursor-pointer"
            >
              Hủy
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-1.5 rounded bg-[#1878F5] hover:bg-[#2F8CFF] text-white font-semibold cursor-pointer disabled:opacity-50"
            >
              {submitting ? 'Đang tạo...' : 'Tạo Project'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

interface CloneProjectModalProps {
  sourceProject: GameProjectRecord | null;
  onClose: () => void;
  onSubmit: (req: CloneProjectRequest) => Promise<void>;
}

export const CloneProjectModal: React.FC<CloneProjectModalProps> = ({
  sourceProject,
  onClose,
  onSubmit,
}) => {
  const [newName, setNewName] = useState(sourceProject ? `${sourceProject.name} (Clone)` : '');
  const [newSlug, setNewSlug] = useState(sourceProject ? `${sourceProject.slug}-clone` : '');
  const [newPkg, setNewPkg] = useState(sourceProject?.gamePackageId || '');
  const [newExe, setNewExe] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!sourceProject) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!newName.trim() || !newSlug.trim()) {
      setError('Tên và Slug bản sao là bắt buộc.');
      return;
    }
    setSubmitting(true);
    try {
      await onSubmit({
        sourceProjectId: sourceProject.id,
        newSlug: newSlug.trim(),
        newName: newName.trim(),
        newGamePackageId: newPkg.trim() || null,
        newExeName: newExe.trim() || null,
      });
      onClose();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/65 flex items-center justify-center p-4">
      <div className="w-full max-w-md bg-[#192230] border border-[#2A3647] rounded-xl shadow-2xl overflow-hidden">
        <div className="px-5 py-3.5 bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between">
          <div className="flex items-center gap-2 font-semibold text-sm text-[#EDF3FC]">
            <Copy size={15} className="text-[#3BC38D]" />
            <span>Deep Clone Project</span>
          </div>
          <button onClick={onClose} className="text-[#74869D] hover:text-white cursor-pointer">
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-3 text-xs">
          <div className="p-2.5 rounded bg-[#101722] border border-[#2A3647] text-[#A9B8CC]">
            Nguồn: <strong className="text-[#EDF3FC]">{sourceProject.name}</strong> (
            <code>{sourceProject.slug}</code>)
          </div>
          {error && <div className="p-2.5 rounded fp-badge-error">{error}</div>}

          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">Tên bản sao *</span>
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC]"
            />
          </label>
          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">Slug mới *</span>
            <input
              value={newSlug}
              onChange={(e) => setNewSlug(e.target.value)}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
            />
          </label>
          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">Package ID (tùy chọn đổi game)</span>
            <input
              value={newPkg}
              onChange={(e) => setNewPkg(e.target.value)}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
            />
          </label>
          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">EXE Name mới (để trống để tự sinh)</span>
            <input
              value={newExe}
              onChange={(e) => setNewExe(e.target.value)}
              placeholder={`Auto_${newSlug || 'Clone'}.exe`}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
            />
          </label>

          <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-1.5 rounded bg-[#202B3B] text-[#A9B8CC] cursor-pointer"
            >
              Hủy
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-1.5 rounded bg-[#3BC38D] hover:bg-[#32ad7c] text-[#0B1220] font-semibold cursor-pointer disabled:opacity-50"
            >
              {submitting ? 'Đang nhân bản...' : 'Nhân bản Độc lập'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

interface EditProjectModalProps {
  project: GameProjectRecord | null;
  onClose: () => void;
  onSubmit: (req: UpdateProjectRequest) => Promise<void>;
}

export const EditProjectModal: React.FC<EditProjectModalProps> = ({
  project,
  onClose,
  onSubmit,
}) => {
  const [name, setName] = useState(project?.name || '');
  const [pkg, setPkg] = useState(project?.gamePackageId || '');
  const [version, setVersion] = useState(project?.version || '1.0.0');
  const [exeName, setExeName] = useState(project?.exeName || '');
  const [windowTitle, setWindowTitle] = useState(project?.windowTitle || '');
  const [widthPx, setWidthPx] = useState(project?.targetWidthPx || 1080);
  const [heightPx, setHeightPx] = useState(project?.targetHeightPx || 1920);
  const [bundleAdb, setBundleAdb] = useState(project?.bundleAdb ?? true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!project) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit({
        id: project.id,
        name: name.trim(),
        gamePackageId: pkg.trim(),
        version: version.trim(),
        exeName: exeName.trim(),
        windowTitle: windowTitle.trim(),
        targetWidthPx: widthPx,
        targetHeightPx: heightPx,
        bundleAdb,
      });
      onClose();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/65 flex items-center justify-center p-4">
      <div className="w-full max-w-lg bg-[#192230] border border-[#2A3647] rounded-xl shadow-2xl overflow-hidden">
        <div className="px-5 py-3.5 bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between">
          <div className="flex items-center gap-2 font-semibold text-sm text-[#EDF3FC]">
            <Edit3 size={15} className="text-[#4B9CFF]" />
            <span>Cập nhật Metadata Project ({project.slug})</span>
          </div>
          <button onClick={onClose} className="text-[#74869D] hover:text-white cursor-pointer">
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-3 text-xs">
          {error && <div className="p-2.5 rounded fp-badge-error">{error}</div>}
          <div className="grid grid-cols-2 gap-3">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Tên Project</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Phiên bản</span>
              <input
                value={version}
                onChange={(e) => setVersion(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
              />
            </label>
          </div>
          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">Package ID</span>
            <input
              value={pkg}
              onChange={(e) => setPkg(e.target.value)}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
            />
          </label>
          <div className="grid grid-cols-2 gap-3">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Tên EXE xuất ra</span>
              <input
                value={exeName}
                onChange={(e) => setExeName(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Tiêu đề cửa sổ</span>
              <input
                value={windowTitle}
                onChange={(e) => setWindowTitle(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC]"
              />
            </label>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Rộng (px)</span>
              <input
                type="number"
                value={widthPx}
                onChange={(e) => setWidthPx(Number(e.target.value))}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
              />
            </label>
            <label className="space-y-1 block">
              <span className="text-[#A9B8CC]">Cao (px)</span>
              <input
                type="number"
                value={heightPx}
                onChange={(e) => setHeightPx(Number(e.target.value))}
                className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
              />
            </label>
          </div>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={bundleAdb}
              onChange={(e) => setBundleAdb(e.target.checked)}
              className="accent-[#1878F5]"
            />
            <span className="text-[#EDF3FC]">Đóng gói Portable ADB</span>
          </label>
          <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-1.5 rounded bg-[#202B3B] text-[#A9B8CC] cursor-pointer"
            >
              Hủy
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-1.5 rounded bg-[#1878F5] text-white font-semibold cursor-pointer"
            >
              {submitting ? 'Đang lưu...' : 'Lưu thay đổi'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

interface DeleteProjectModalProps {
  project: GameProjectRecord | null;
  onClose: () => void;
  onConfirm: (projectId: string, removeDiskDir: boolean) => Promise<void>;
}

export const DeleteProjectModal: React.FC<DeleteProjectModalProps> = ({
  project,
  onClose,
  onConfirm,
}) => {
  const [removeDisk, setRemoveDisk] = useState(true);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!project) return null;

  const handleDelete = async () => {
    setDeleting(true);
    setError(null);
    try {
      await onConfirm(project.id, removeDisk);
      onClose();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/65 flex items-center justify-center p-4">
      <div className="w-full max-w-md bg-[#192230] border border-[#F27883]/45 rounded-xl shadow-2xl overflow-hidden">
        <div className="px-5 py-3.5 bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between">
          <div className="flex items-center gap-2 font-semibold text-sm text-[#F27883]">
            <AlertTriangle size={16} />
            <span>Xác nhận Xóa Game Project</span>
          </div>
          <button onClick={onClose} className="text-[#74869D] hover:text-white cursor-pointer">
            <X size={16} />
          </button>
        </div>

        <div className="p-5 space-y-3 text-xs">
          {error && <div className="p-2.5 rounded fp-badge-error">{error}</div>}
          <p className="text-[#EDF3FC]">
            Bạn sắp xóa dự án <strong>{project.name}</strong> (<code>{project.slug}</code>) khỏi
            SQLite. Thao tác này xóa sạch workflows, tasks, templates, accounts và jobs thuộc dự án
            này (không ảnh hưởng tới các dự án khác).
          </p>
          <label className="flex items-center gap-2 p-2.5 rounded bg-[#101722] border border-[#2A3647] cursor-pointer">
            <input
              type="checkbox"
              checked={removeDisk}
              onChange={(e) => setRemoveDisk(e.target.checked)}
              className="accent-[#F27883]"
            />
            <span className="text-[#A9B8CC]">Xóa an toàn thư mục dự án trên đĩa</span>
          </label>
          <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
            <button
              onClick={onClose}
              className="px-3.5 py-1.5 rounded bg-[#202B3B] text-[#A9B8CC] cursor-pointer"
            >
              Hủy
            </button>
            <button
              onClick={handleDelete}
              disabled={deleting}
              className="px-4 py-1.5 rounded bg-[#F27883] text-[#0B1220] font-bold flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Trash2 size={13} />
              <span>{deleting ? 'Đang xóa...' : 'Xóa Project'}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

interface LegacyMigrateModalProps {
  isOpen: boolean;
  onClose: () => void;
  onMigrate: (legacyDir: string) => Promise<void>;
}

export const LegacyMigrateModal: React.FC<LegacyMigrateModalProps> = ({
  isOpen,
  onClose,
  onMigrate,
}) => {
  const [dirPath, setDirPath] = useState('projects/poc/jx2-mobile');
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleRun = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setRunning(true);
    try {
      await onMigrate(dirPath.trim());
      onClose();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/65 flex items-center justify-center p-4">
      <div className="w-full max-w-md bg-[#192230] border border-[#2A3647] rounded-xl shadow-2xl overflow-hidden">
        <div className="px-5 py-3.5 bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between">
          <div className="flex items-center gap-2 font-semibold text-sm text-[#EDF3FC]">
            <DatabaseBackup size={15} className="text-[#E7B15B]" />
            <span>Import / Migrate Legacy Project (Không phá hủy gốc)</span>
          </div>
          <button onClick={onClose} className="text-[#74869D] hover:text-white cursor-pointer">
            <X size={16} />
          </button>
        </div>
        <form onSubmit={handleRun} className="p-5 space-y-3 text-xs">
          {error && <div className="p-2.5 rounded fp-badge-error">{error}</div>}
          <p className="text-[#A9B8CC]">
            Chuyển đổi dự án legacy trên đĩa vào SQLite Multi-Project P2 mà vẫn giữ nguyên vẹn thư mục
            gốc.
          </p>
          <label className="space-y-1 block">
            <span className="text-[#A9B8CC]">Đường dẫn thư mục dự án cũ</span>
            <input
              value={dirPath}
              onChange={(e) => setDirPath(e.target.value)}
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
            />
          </label>
          <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-1.5 rounded bg-[#202B3B] text-[#A9B8CC] cursor-pointer"
            >
              Đóng
            </button>
            <button
              type="submit"
              disabled={running}
              className="px-4 py-1.5 rounded bg-[#E7B15B] text-[#0B1220] font-semibold cursor-pointer disabled:opacity-50"
            >
              {running ? 'Đang Import...' : 'Chạy Import'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
