import { invoke } from '@tauri-apps/api/core';
import { DeviceSummary, DeviceDetail, StepEvent, ValidationReport } from '../types';

export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
};

export interface AppStatus {
  version: string;
  adb_available: boolean;
  adb_version: string;
  adb_path: string;
  db_status: string;
  db_path: string;
  devices_count: number;
  is_portable: boolean;
}

export interface ExecuteWorkflowResponse {
  job_id: string;
  state: string;
  events_count: number;
}

export interface NodeTestResult {
  success: boolean;
  duration_ms: number;
  message: string;
  point_tapped?: { x: number; y: number } | null;
}

export const api = {
  async windowMinimize(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_minimize');
    } catch (e) {
      console.warn('Failed to minimize window:', e);
    }
  },

  async windowToggleMaximize(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_toggle_maximize');
    } catch (e) {
      console.warn('Failed to toggle maximize:', e);
    }
  },

  async windowClose(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_close');
    } catch (e) {
      console.warn('Failed to close window:', e);
    }
  },

  async getStatus(): Promise<AppStatus | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<AppStatus>('app_get_status');
    } catch (e) {
      console.warn('Failed to call app_get_status:', e);
      return null;
    }
  },

  async listDevices(): Promise<DeviceSummary[]> {
    if (!isTauri()) return [];
    try {
      return await invoke<DeviceSummary[]>('list_devices');
    } catch (e) {
      console.warn('Failed to call list_devices:', e);
      return [];
    }
  },

  async inspectDevice(deviceId: string): Promise<DeviceDetail | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<DeviceDetail>('inspect_device', { deviceId });
    } catch (e) {
      console.warn('Failed to call inspect_device:', e);
      return null;
    }
  },

  async captureDevice(deviceId: string): Promise<string | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<string>('capture_device', { deviceId });
    } catch (e) {
      console.warn('Failed to call capture_device:', e);
      return null;
    }
  },

  async validateWorkflow(workflowJson: string): Promise<ValidationReport> {
    if (!isTauri()) {
      try {
        const doc = JSON.parse(workflowJson);
        const hasStart = doc.graph?.nodes?.some((n: any) => n.type === 'flow.start');
        const hasEnd = doc.graph?.nodes?.some((n: any) => n.type === 'flow.end');
        const errors: string[] = [];
        if (!hasStart) errors.push("Thiếu node 'flow.start' (Bắt đầu)");
        if (!hasEnd) errors.push("Thiếu node 'flow.end' (Kết thúc)");
        return { is_valid: errors.length === 0, errors, warnings: [] };
      } catch (err: any) {
        return { is_valid: false, errors: [err.message], warnings: [] };
      }
    }
    return await invoke<ValidationReport>('validate_workflow', { workflowJson });
  },

  async executeWorkflow(workflowJson: string, deviceId: string, mock: boolean = true): Promise<ExecuteWorkflowResponse> {
    if (!isTauri()) {
      return { job_id: 'mock-web-job', state: 'Succeeded', events_count: 8 };
    }
    return await invoke<ExecuteWorkflowResponse>('execute_workflow', { workflowJson, deviceId, mock });
  },

  async executeTestNode(
    deviceId: string,
    nodeType: string,
    configJson: string
  ): Promise<NodeTestResult> {
    if (!isTauri()) {
      return {
        success: true,
        duration_ms: 45,
        message: `Node '${nodeType}' chạy thành công trong môi trường mô phỏng`,
        point_tapped: { x: 242.0, y: 428.0 },
      };
    }
    return await invoke<NodeTestResult>('execute_test_node', {
      deviceId,
      nodeType,
      configJson,
    });
  },

  async getAuditEvents(jobId?: string): Promise<StepEvent[]> {
    if (!isTauri()) return [];
    try {
      return await invoke<StepEvent[]>('get_audit_events', { jobId });
    } catch (e) {
      console.warn('Failed to call get_audit_events:', e);
      return [];
    }
  },

  async saveWorkflowDraft(workflowId: string, name: string, draftJson: string): Promise<boolean> {
    if (!isTauri()) return true;
    try {
      return await invoke<boolean>('save_workflow_draft', { workflowId, name, draftJson });
    } catch (e) {
      console.warn('Failed to call save_workflow_draft:', e);
      return false;
    }
  },

  async getWorkflowDraft(workflowId: string): Promise<string | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<string | null>('get_workflow_draft', { workflowId });
    } catch (e) {
      console.warn('Failed to call get_workflow_draft:', e);
      return null;
    }
  },
};
