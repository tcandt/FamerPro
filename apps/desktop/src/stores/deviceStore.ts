import { create } from 'zustand';
import { DeviceSummary, DeviceDetail } from '../types';
import { api, isTauri } from '../lib/api';

interface DeviceState {
  devices: DeviceSummary[];
  selectedDeviceId: string | null;
  selectedDeviceDetail: DeviceDetail | null;
  capturedScreenshot: string | null;
  isLoadingDevices: boolean;
  isInspecting: boolean;
  appStatus: any | null;
  showDemoDevices: boolean;

  setDevices: (devices: DeviceSummary[]) => void;
  selectDevice: (id: string | null) => void;
  setSelectedDeviceDetail: (detail: DeviceDetail | null) => void;
  setCapturedScreenshot: (dataUrl: string | null) => void;
  setIsLoadingDevices: (loading: boolean) => void;
  setIsInspecting: (inspecting: boolean) => void;
  setShowDemoDevices: (show: boolean) => void;
  fetchDevices: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  captureCurrentDevice: () => Promise<void>;
}

const mockDevices: DeviceSummary[] = [
  {
    id: 'mock-pixel-6',
    adb_serial: 'mock-pixel-6',
    alias: 'Pixel 6 — Máy Mẫu 01',
    android_api: 33,
    model: 'Pixel_6',
    width_px: 1080,
    height_px: 2400,
    rotation_deg: 0,
    connection_state: 'ready',
    updated_at: new Date().toISOString(),
    is_mock: true,
  },
  {
    id: 'mock-galaxy-s22',
    adb_serial: '192.168.1.105:5555',
    alias: 'Galaxy S22 — Máy Mẫu 02',
    android_api: 31,
    model: 'SM-S901B',
    width_px: 1080,
    height_px: 2340,
    rotation_deg: 0,
    connection_state: 'ready',
    updated_at: new Date().toISOString(),
    is_mock: true,
  },
  {
    id: 'mock-xiaomi-12',
    adb_serial: '192.168.1.112:5555',
    alias: 'Xiaomi 12 — Máy Mẫu 03',
    android_api: 29,
    model: '2201123G',
    width_px: 1080,
    height_px: 2400,
    rotation_deg: 0,
    connection_state: 'unauthorized',
    updated_at: new Date().toISOString(),
    is_mock: true,
  },
];

export const useDeviceStore = create<DeviceState>((set, get) => ({
  devices: mockDevices,
  selectedDeviceId: 'mock-pixel-6',
  selectedDeviceDetail: {
    summary: mockDevices[0],
    battery_level: 95,
    is_screen_on: true,
    ip_address: '127.0.0.1',
    installed_packages: ['com.blackhole.jx2mobile', 'com.android.chrome'],
  },
  capturedScreenshot: null,
  isLoadingDevices: false,
  isInspecting: false,
  appStatus: null,
  showDemoDevices: true,

  setDevices: (devices) => set({ devices }),
  selectDevice: async (id) => {
    set({ selectedDeviceId: id });
    if (id && isTauri()) {
      try {
        const detail = await api.inspectDevice(id);
        if (detail) {
          set({ selectedDeviceDetail: detail });
        }
      } catch (e) {
        console.warn('Failed to inspect device:', e);
      }
    } else {
      const matchMock = get().devices.find((d) => d.id === id);
      if (matchMock) {
        set({
          selectedDeviceDetail: {
            summary: matchMock,
            battery_level: 95,
            is_screen_on: true,
            ip_address: '127.0.0.1',
            installed_packages: ['com.blackhole.jx2mobile', 'com.android.chrome'],
          },
        });
      }
    }
  },
  setSelectedDeviceDetail: (detail) => set({ selectedDeviceDetail: detail }),
  setCapturedScreenshot: (dataUrl) => set({ capturedScreenshot: dataUrl }),
  setIsLoadingDevices: (loading) => set({ isLoadingDevices: loading }),
  setIsInspecting: (inspecting) => set({ isInspecting: inspecting }),
  setShowDemoDevices: (show) => {
    set({ showDemoDevices: show });
    get().fetchDevices();
  },

  fetchStatus: async () => {
    const status = await api.getStatus();
    if (status) {
      set({ appStatus: status });
    }
  },

  fetchDevices: async () => {
    if (!isTauri()) return;
    set({ isLoadingDevices: true });
    try {
      const realDevices = await api.listDevices();
      const showDemo = get().showDemoDevices;
      const markedReal = realDevices.map((d) => ({ ...d, is_mock: false }));

      if (markedReal.length > 0) {
        const uniqueMock = mockDevices.filter((m) => !markedReal.some((r) => r.id === m.id || r.adb_serial === m.adb_serial));
        const combined = showDemo ? [...markedReal, ...uniqueMock] : markedReal;
        const currentSelected = get().selectedDeviceId;
        // If current selection is null or a mock device, prefer the first real LIVE device (e.g. emulator-5554)
        const isCurrentReal = markedReal.some((d) => d.id === currentSelected);
        const nextSelected = isCurrentReal ? currentSelected : markedReal[0].id;

        set({
          devices: combined,
          selectedDeviceId: nextSelected,
        });

        if (nextSelected) {
          try {
            const detail = await api.inspectDevice(nextSelected);
            if (detail) {
              set({ selectedDeviceDetail: detail });
            }
          } catch (e) {
            console.warn('Failed to inspect live device:', e);
          }
        }
      } else {
        set({
          devices: showDemo ? mockDevices : [],
          selectedDeviceId: showDemo ? mockDevices[0]?.id || null : null,
          selectedDeviceDetail: showDemo ? {
            summary: mockDevices[0],
            battery_level: 95,
            is_screen_on: true,
            ip_address: '127.0.0.1',
            installed_packages: ['com.blackhole.jx2mobile', 'com.android.chrome'],
          } : null,
        });
      }
    } catch (err) {
      console.warn('Failed to fetch devices:', err);
    } finally {
      set({ isLoadingDevices: false });
    }
  },

  captureCurrentDevice: async () => {
    const devId = useDeviceStore.getState().selectedDeviceId;
    if (!devId) return;
    const b64 = await api.captureDevice(devId);
    if (b64) {
      set({ capturedScreenshot: b64 });
    }
  },
}));
