import { create } from 'zustand';
import { DeviceSummary, DeviceDetail, AppPackageInfo, LdPlayerInstance, ScanRangeResult } from '../types';
import { api, isTauri } from '../lib/api';

interface DeviceState {
  devices: DeviceSummary[];
  selectedDeviceId: string | null;
  selectedDeviceDetail: DeviceDetail | null;
  packages: AppPackageInfo[];
  selectedPackageName: string | null;
  ldInstances: LdPlayerInstance[];
  ldPath: string | null;
  capturedScreenshot: string | null;
  isLoadingDevices: boolean;
  isInspecting: boolean;
  isLoadingPackages: boolean;
  isLoadingLd: boolean;
  appStatus: any | null;

  setDevices: (devices: DeviceSummary[]) => void;
  selectDevice: (id: string | null) => Promise<void>;
  setSelectedDeviceDetail: (detail: DeviceDetail | null) => void;
  setSelectedPackageName: (pkg: string | null) => void;
  setCapturedScreenshot: (dataUrl: string | null) => void;
  setIsLoadingDevices: (loading: boolean) => void;
  setIsInspecting: (inspecting: boolean) => void;

  fetchDevices: (autoSelect?: boolean) => Promise<void>;
  fetchStatus: () => Promise<void>;
  inspectDevice: (deviceId: string) => Promise<void>;
  loadPackages: (deviceId: string, userOnly?: boolean) => Promise<void>;
  launchApp: (deviceId?: string, packageName?: string) => Promise<string>;
  stopApp: (deviceId?: string, packageName?: string) => Promise<string>;
  fetchLdInstances: () => Promise<void>;
  ldAction: (action: string, index?: number) => Promise<string>;
  connectEndpoint: (endpoint: string) => Promise<string>;
  disconnectEndpoint: (endpoint: string) => Promise<string>;
  scanRange: (subnet?: string, port?: number, emulatorPorts?: boolean) => Promise<ScanRangeResult>;
  autoDiscover: () => Promise<void>;
  captureCurrentDevice: () => Promise<void>;
}

export const useDeviceStore = create<DeviceState>((set, get) => ({
  devices: [],
  selectedDeviceId: null,
  selectedDeviceDetail: null,
  packages: [],
  selectedPackageName: null,
  ldInstances: [],
  ldPath: null,
  capturedScreenshot: null,
  isLoadingDevices: false,
  isInspecting: false,
  isLoadingPackages: false,
  isLoadingLd: false,
  appStatus: null,

  setDevices: (devices) => set({ devices }),

  setSelectedDeviceDetail: (detail) => set({ selectedDeviceDetail: detail }),
  setSelectedPackageName: (pkg) => set({ selectedPackageName: pkg }),
  setCapturedScreenshot: (dataUrl) => set({ capturedScreenshot: dataUrl }),
  setIsLoadingDevices: (loading) => set({ isLoadingDevices: loading }),
  setIsInspecting: (inspecting) => set({ isInspecting: inspecting }),

  selectDevice: async (id: string | null) => {
    set({ selectedDeviceId: id });
    if (!id) {
      set({ selectedDeviceDetail: null, packages: [], selectedPackageName: null });
      return;
    }
    await Promise.all([
      get().inspectDevice(id),
      get().loadPackages(id, true),
    ]);
  },

  inspectDevice: async (deviceId: string) => {
    if (!isTauri() || !deviceId) return;
    set({ isInspecting: true });
    try {
      const detail = await api.inspectDevice(deviceId);
      if (detail) {
        set({ selectedDeviceDetail: detail });
      }
    } catch (e) {
      console.warn('Failed to inspect device:', e);
    } finally {
      set({ isInspecting: false });
    }
  },

  loadPackages: async (deviceId: string, userOnly: boolean = true) => {
    if (!isTauri() || !deviceId) return;
    set({ isLoadingPackages: true });
    try {
      let pkgs = await api.listDevicePackages(deviceId, userOnly);
      if (pkgs.length === 0 && userOnly) {
        // Fallback to all packages if third-party app list is empty
        pkgs = await api.listDevicePackages(deviceId, false);
      }
      set({ packages: pkgs });
      // Auto select first game or first app
      if (pkgs.length > 0) {
        const currentSelected = get().selectedPackageName;
        const exists = pkgs.some((p) => p.package_name === currentSelected);
        if (!exists) {
          const preferred = pkgs.find((p) => p.is_game) || pkgs[0];
          set({ selectedPackageName: preferred.package_name });
        }
      } else {
        set({ selectedPackageName: null });
      }
    } catch (e) {
      console.warn('Failed to load packages:', e);
    } finally {
      set({ isLoadingPackages: false });
    }
  },

  launchApp: async (deviceId?: string, packageName?: string) => {
    const devId = deviceId || get().selectedDeviceId;
    const pkg = packageName || get().selectedPackageName;
    if (!devId) throw new Error('Chưa chọn thiết bị');
    if (!pkg) throw new Error('Chưa chọn ứng dụng để mở');
    return await api.launchDeviceApp(devId, pkg);
  },

  stopApp: async (deviceId?: string, packageName?: string) => {
    const devId = deviceId || get().selectedDeviceId;
    const pkg = packageName || get().selectedPackageName;
    if (!devId) throw new Error('Chưa chọn thiết bị');
    if (!pkg) throw new Error('Chưa chọn ứng dụng để tắt');
    return await api.stopDeviceApp(devId, pkg);
  },

  fetchLdInstances: async () => {
    if (!isTauri()) return;
    set({ isLoadingLd: true });
    try {
      const [list, path] = await Promise.all([
        api.ldplayerListInstances(),
        api.ldplayerDetectPath(),
      ]);
      set({ ldInstances: list, ldPath: path });
    } catch (e) {
      console.warn('Failed to fetch LDPlayer instances:', e);
    } finally {
      set({ isLoadingLd: false });
    }
  },

  ldAction: async (action: string, index?: number) => {
    const res = await api.ldplayerAction(action, index);
    // Refresh LD instances and devices after action
    setTimeout(() => {
      get().fetchLdInstances();
      get().fetchDevices();
    }, 1500);
    return res;
  },

  connectEndpoint: async (endpoint: string) => {
    const res = await api.adbConnectEndpoint(endpoint);
    await get().fetchDevices(true);
    return res;
  },

  disconnectEndpoint: async (endpoint: string) => {
    const res = await api.adbDisconnectEndpoint(endpoint);
    await get().fetchDevices();
    return res;
  },

  scanRange: async (subnet?: string, port?: number, emulatorPorts?: boolean) => {
    const res = await api.adbScanRange(subnet, port, emulatorPorts);
    await get().fetchDevices(true);
    return res;
  },

  autoDiscover: async () => {
    await api.autoDiscoverDevices();
    await get().fetchDevices(true);
  },

  fetchStatus: async () => {
    const status = await api.getStatus();
    if (status) {
      set({ appStatus: status });
    }
  },

  fetchDevices: async (autoSelect: boolean = true) => {
    if (!isTauri()) return;
    set({ isLoadingDevices: true });
    try {
      const realDevices = await api.listDevices();
      const currentSelected = get().selectedDeviceId;
      const currentDevice = realDevices.find((d) => d.id === currentSelected);

      let nextSelected: string | null = null;
      if (currentDevice && currentDevice.connection_state === 'ready') {
        nextSelected = currentSelected;
      } else if (autoSelect && realDevices.length > 0) {
        // Prefer first ready device, otherwise current if present, otherwise first device
        const ready = realDevices.find((d) => d.connection_state === 'ready');
        nextSelected = ready ? ready.id : (currentDevice ? currentDevice.id : realDevices[0].id);
      } else if (currentDevice) {
        nextSelected = currentSelected;
      }

      set({
        devices: realDevices,
        selectedDeviceId: nextSelected,
      });

      // Only trigger heavy inspect and package listing if selection newly changed or missing detail
      if (nextSelected && (nextSelected !== currentSelected || !get().selectedDeviceDetail)) {
        await Promise.all([
          get().inspectDevice(nextSelected),
          get().loadPackages(nextSelected, true),
        ]);
      } else if (!nextSelected) {
        set({ selectedDeviceDetail: null, packages: [], selectedPackageName: null });
      }
    } catch (err) {
      console.warn('Failed to fetch devices:', err);
    } finally {
      set({ isLoadingDevices: false });
    }
  },

  captureCurrentDevice: async () => {
    const devId = get().selectedDeviceId;
    if (!devId) return;
    const b64 = await api.captureDevice(devId);
    if (b64) {
      set({ capturedScreenshot: b64 });
    }
  },
}));
