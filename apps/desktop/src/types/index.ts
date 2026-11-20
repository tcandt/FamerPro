export type ConnectionState =
  | 'disconnected'
  | 'unauthorized'
  | 'authorizing'
  | 'ready'
  | 'busy'
  | 'recovering'
  | 'blocked';

export interface DeviceSummary {
  id: string;
  adb_serial: string;
  alias: string;
  android_api?: number;
  model?: string;
  width_px: number;
  height_px: number;
  rotation_deg: number;
  connection_state: ConnectionState;
  updated_at: string;
}

export interface DeviceDetail {
  summary: DeviceSummary;
  battery_level?: number;
  is_screen_on: boolean;
  ip_address?: string;
  installed_packages: string[];
}

export interface WorkflowNodeData extends Record<string, unknown> {
  label: string;
  nodeType: string;
  category: 'flow' | 'app' | 'vision' | 'action' | 'verify' | 'state' | 'logic';
  config: Record<string, any>;
  status?: 'idle' | 'running' | 'success' | 'failed';
  timeoutMs?: number;
}

export interface WorkflowEdgeData {
  portType: 'flow' | 'data' | 'boolean';
}

export interface ValidationReport {
  is_valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface StepEvent {
  id: string;
  job_id: string;
  node_id: string;
  event_type: 'command_sent' | 'device_ack' | 'verification_passed' | 'verification_failed' | 'checkpoint_saved';
  attempt_no: number;
  at: string;
  payload_json: string;
}

export interface AppStatus {
  version: string;
  adb_available: boolean;
  adb_version: string;
  db_status: string;
  devices_count: number;
}
