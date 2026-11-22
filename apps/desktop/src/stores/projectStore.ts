import { create } from 'zustand';
import { api, isTauri } from '../lib/api';
import type {
  CloneProjectRequest,
  CreateProjectRequest,
  GameProjectDetail,
  GameProjectRecord,
  GameProjectSummary,
  UpdateProjectRequest,
} from '../types';

const STORAGE_ACTIVE_PROJECT_KEY = 'farmerpro.studio.activeProjectId';

interface ProjectState {
  projects: GameProjectSummary[];
  activeProjectId: string | null;
  activeProjectDetail: GameProjectDetail | null;
  isLoadingProjects: boolean;
  isLoadingDetail: boolean;
  lastError: string | null;

  fetchProjects: (autoSelect?: boolean) => Promise<void>;
  selectProject: (projectId: string | null) => Promise<void>;
  refreshActiveProject: () => Promise<void>;
  createProject: (req: CreateProjectRequest) => Promise<GameProjectRecord>;
  updateProject: (req: UpdateProjectRequest) => Promise<GameProjectRecord>;
  cloneProject: (req: CloneProjectRequest) => Promise<GameProjectRecord>;
  deleteProject: (projectId: string, removeDiskDir?: boolean) => Promise<boolean>;
  clearError: () => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  activeProjectId:
    typeof window !== 'undefined' ? window.localStorage.getItem(STORAGE_ACTIVE_PROJECT_KEY) : null,
  activeProjectDetail: null,
  isLoadingProjects: false,
  isLoadingDetail: false,
  lastError: null,

  clearError: () => set({ lastError: null }),

  fetchProjects: async (autoSelect: boolean = true) => {
    if (!isTauri()) {
      set({ projects: [], isLoadingProjects: false });
      return;
    }
    set({ isLoadingProjects: true, lastError: null });
    try {
      const list = await api.studioListProjects();
      set({ projects: list });

      if (autoSelect) {
        const cur = get().activeProjectId;
        const stillExists = cur && list.some((p) => p.project.id === cur);
        const nextId = stillExists ? cur : list[0]?.project.id || null;
        if (nextId !== cur || (nextId && !get().activeProjectDetail)) {
          await get().selectProject(nextId);
        }
      }
    } catch (err: any) {
      set({ lastError: err?.message || String(err) });
    } finally {
      set({ isLoadingProjects: false });
    }
  },

  selectProject: async (projectId: string | null) => {
    set({ activeProjectId: projectId });
    if (typeof window !== 'undefined') {
      if (projectId) {
        window.localStorage.setItem(STORAGE_ACTIVE_PROJECT_KEY, projectId);
      } else {
        window.localStorage.removeItem(STORAGE_ACTIVE_PROJECT_KEY);
      }
    }

    if (!projectId || !isTauri()) {
      set({ activeProjectDetail: null });
      return;
    }

    set({ isLoadingDetail: true, lastError: null });
    try {
      const detail = await api.studioGetProjectDetail(projectId);
      set({ activeProjectDetail: detail });
    } catch (err: any) {
      set({ lastError: err?.message || String(err) });
    } finally {
      set({ isLoadingDetail: false });
    }
  },

  refreshActiveProject: async () => {
    const activeId = get().activeProjectId;
    await get().fetchProjects(false);
    if (activeId) {
      await get().selectProject(activeId);
    }
  },

  createProject: async (req: CreateProjectRequest) => {
    const rec = await api.studioCreateProject(req);
    await get().fetchProjects(false);
    await get().selectProject(rec.id);
    return rec;
  },

  updateProject: async (req: UpdateProjectRequest) => {
    const rec = await api.studioUpdateProject(req);
    await get().refreshActiveProject();
    return rec;
  },

  cloneProject: async (req: CloneProjectRequest) => {
    const rec = await api.studioCloneProject(req);
    await get().fetchProjects(false);
    await get().selectProject(rec.id);
    return rec;
  },

  deleteProject: async (projectId: string, removeDiskDir: boolean = true) => {
    const deleted = await api.studioDeleteProject(projectId, removeDiskDir);
    if (deleted) {
      const wasActive = get().activeProjectId === projectId;
      await get().fetchProjects(wasActive);
    }
    return deleted;
  },
}));
